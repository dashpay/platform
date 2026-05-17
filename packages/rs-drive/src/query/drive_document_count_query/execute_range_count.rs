//! Range execution paths for the count query.
//!
//! Three executors all keyed on a `range_countable: true` index:
//! - [`DriveDocumentCountQuery::execute_range_count_no_proof`] — Rust-
//!   side walk of the property-name `ProvableCountTree`'s children,
//!   returning per-(in_key, key) entries (or a single sum) without a
//!   proof.
//! - [`DriveDocumentCountQuery::execute_aggregate_count_with_proof`] —
//!   grovedb `AggregateCountOnRange` proof, returning a single u64.
//! - [`DriveDocumentCountQuery::execute_distinct_count_with_proof`] —
//!   regular range proof against the `ProvableCountTree`, returning
//!   per-key `KVCount` ops bound to the merk root.
//!
//! Point-lookup execution (Equal/In with no range) lives in
//! [`super::execute_point_lookup`](super::execute_point_lookup).
//!
//! Whole module is gated `feature = "server"` via the parent's
//! `pub mod execute_range_count;` declaration.

use super::super::conditions::{WhereClause, WhereOperator};
use super::{DriveDocumentCountQuery, SplitCountEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;
use grovedb_costs::CostContext;

/// Pagination + ordering knobs for `execute_range_count_no_proof`.
///
/// Mirrors the protobuf request fields on
/// `GetDocumentsCountRequestV0` so the drive-abci handler can pass them
/// through unmodified. `distinct = false` collapses the range walk to a
/// single summed entry; `distinct = true` returns one entry per distinct
/// property value within the range.
#[derive(Debug, Clone, Default)]
pub struct RangeCountOptions {
    /// When `true`, return one [`SplitCountEntry`] per distinct property
    /// value within the range. When `false`, return a single entry
    /// (empty `key`) summing all per-value counts.
    pub distinct: bool,
    /// Maximum number of entries to return. Only meaningful when
    /// `distinct = true`. `None` means no limit.
    ///
    /// To paginate, callers narrow the range itself (`color >
    /// <last-key-from-previous-page>`). There's no cursor field
    /// because a single-`bytes` cursor would be ambiguous for
    /// compound (`In + range + distinct`) queries whose natural sort
    /// is `(in_key, key)`, and range narrowing has the same
    /// expressivity for the simple cases.
    pub limit: Option<u32>,
    /// Sort order for distinct entries. `true` (default) is ascending by
    /// serialized key bytes. Ignored when `distinct = false`.
    pub order_by_ascending: bool,
}

impl DriveDocumentCountQuery<'_> {
    /// Executes a range-aware count query against a `range_countable`
    /// index. Path layout is `[contract_doc, doctype, prefix...,
    /// range_prop_name]`, whose children are the per-value
    /// `CountTree` leaves keyed by the range property's serialized
    /// value.
    ///
    /// The caller picks the index via
    /// [`Self::find_range_countable_index_for_where_clauses`]; this
    /// method assumes:
    /// - `self.index.range_countable == true`
    /// - All `Equal` / `In` where clauses cover the index prefix
    /// - Exactly one range-operator where clause hits the index's last
    ///   property
    ///
    /// ## Execution strategies by mode
    ///
    /// - **Flat summed** (no `In`, `distinct = false`): single
    ///   `query_aggregate_count` call against the merk-level
    ///   `AggregateCountOnRange` primitive. O(log n).
    /// - **Compound summed** (`In` on prefix, `distinct = false`):
    ///   per-In-value fan-out — one `query_aggregate_count` call per
    ///   matched In branch, summed in Rust. Bounded by the In
    ///   array's 100-element cap (enforced by
    ///   [`WhereClause::in_values`]) times O(log n), so worst-case
    ///   work is 100 × O(log n) regardless of how many documents
    ///   the range actually matches. Closes the request-amplification
    ///   surface a pre-fix walk-and-sum implementation had: that
    ///   path materialized every matched `(in_key, key)` element
    ///   even though the response was still a single aggregate
    ///   `u64`.
    /// - **Distinct mode** (`distinct = true`, with or without
    ///   `In` on prefix): walks the unified
    ///   [`Self::distinct_count_path_query`] and emits one entry per
    ///   matched `(in_key, key)` pair. The path query carries
    ///   `options.limit` (clamped to `max_query_limit` upstream by
    ///   the dispatcher) and `options.order_by_ascending`, so
    ///   per-query work is O(limit × log n). Cross-fork aggregation
    ///   is intentionally NOT performed server-side; callers reduce
    ///   by `key` client-side if they want a flat histogram. See the
    ///   book chapter ("No-Merge Compound Semantics") for the rationale.
    ///
    /// ## Returned entry shape
    ///
    /// When `options.distinct = false`, returns a single entry with
    /// `in_key = None`, empty `key`, and `count` equal to the sum of
    /// all matched per-value counts. When `options.distinct = true`,
    /// returns one entry per emitted `(in_key, key)` pair, after
    /// applying `order_by_ascending` and `limit` over the
    /// lexicographic `(in_key, key)` tuple.
    pub fn execute_range_count_no_proof(
        &self,
        drive: &Drive,
        options: &RangeCountOptions,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let drive_version = &platform_version.drive;
        let has_in_on_prefix = self
            .where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);

        // Summed mode (both flat and compound `In + range`) goes
        // through grovedb's `AggregateCountOnRange` primitive
        // (`query_aggregate_count`), bounding per-query work to
        // O(log n) per merk-tree fan-out. Compound mode loops over
        // the In values (≤100 per the `in_values()` validator cap
        // in `WhereClause::in_values()`) and issues one aggregate
        // call per value, then sums the results — total bound is
        // O(|In| × log n), independent of how many documents
        // actually match the range.
        //
        // The pre-fix walk-and-sum path materialized every matched
        // `(in_key, key)` element via `query_raw` to sum them in
        // Rust. With one broad range × 100 In values that scans
        // potentially millions of CountTree elements even though
        // the response is still a single aggregate `u64` — a
        // classic request-amplification surface on a public DAPI
        // endpoint. The per-In fan-out closes that surface.
        if !options.distinct {
            if has_in_on_prefix {
                let in_clause = self
                    .where_clauses
                    .iter()
                    .find(|wc| wc.operator == WhereOperator::In)
                    .ok_or_else(|| {
                        Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                            "compound summed range count path requires an `in` clause; \
                             dispatcher bug if reached without one",
                        ))
                    })?;
                // `in_values()` enforces non-empty, ≤100, no-duplicates
                // — same defensive cap every other In consumer in
                // drive uses. Without it a single 64 MiB gRPC request
                // could schedule arbitrarily many backend aggregate
                // reads.
                let in_values = in_clause.in_values().into_data_with_error()??;
                let other_clauses: Vec<WhereClause> = self
                    .where_clauses
                    .iter()
                    .filter(|wc| wc.operator != WhereOperator::In)
                    .cloned()
                    .collect();

                let mut total: u64 = 0;
                let mut seen_keys: std::collections::BTreeSet<Vec<u8>> =
                    std::collections::BTreeSet::new();
                for value in in_values.iter() {
                    // Dedupe by serialized canonical key, not by raw
                    // Value, so that distinct DPP values that
                    // collapse to the same indexed bytes don't get
                    // double-counted. `in_values()` already rejects
                    // raw-Value duplicates, but this is defense-in-
                    // depth against future Value variants that
                    // serialize identically (e.g. integer vs
                    // float-with-zero-fraction).
                    let key_bytes = self.document_type.serialize_value_for_key(
                        in_clause.field.as_str(),
                        value,
                        platform_version,
                    )?;
                    if !seen_keys.insert(key_bytes) {
                        continue;
                    }

                    // Per-In-value query: replace the In clause with
                    // an Equal on the specific value. The resulting
                    // shape is flat (no In, Equal-prefix + range
                    // terminator), so `aggregate_count_path_query`
                    // accepts it and `query_aggregate_count` walks
                    // boundary nodes in O(log n).
                    let mut clauses_for_value = other_clauses.clone();
                    clauses_for_value.push(WhereClause {
                        field: in_clause.field.clone(),
                        operator: WhereOperator::Equal,
                        value: value.clone(),
                    });
                    let per_value_query = DriveDocumentCountQuery {
                        document_type: self.document_type,
                        contract_id: self.contract_id,
                        document_type_name: self.document_type_name.clone(),
                        index: self.index,
                        where_clauses: clauses_for_value,
                    };
                    let path_query =
                        per_value_query.aggregate_count_path_query(platform_version)?;
                    // Destructure the `CostContext` explicitly rather than
                    // calling `.unwrap()` on it: `CostContext::unwrap` is
                    // infallible (it just drops the cost field), but the
                    // visual pattern collides with `Option/Result::unwrap`
                    // and makes review noisier. Cost is discarded here
                    // because the per-mode dispatcher in `drive_dispatcher`
                    // wraps these executors with its own fee accounting —
                    // see the module-level docstring.
                    let CostContext { value, cost: _ } = drive.grove.query_aggregate_count(
                        &path_query,
                        transaction,
                        &drive_version.grove_version,
                    );
                    let count = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
                    total = total.saturating_add(count);
                }
                return Ok(vec![SplitCountEntry {
                    in_key: None,
                    key: Vec::new(),
                    // Range-summed total derived from the executor's
                    // per-In aggregate fan-out — verified count.
                    count: Some(total),
                }]);
            }
            // Flat summed (no In on prefix): single aggregate read.
            let path_query = self.aggregate_count_path_query(platform_version)?;
            // See In-fan-out branch above for the destructure rationale.
            let CostContext { value, cost: _ } = drive.grove.query_aggregate_count(
                &path_query,
                transaction,
                &drive_version.grove_version,
            );
            let count = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
            return Ok(vec![SplitCountEntry {
                in_key: None,
                key: Vec::new(),
                // Single `AggregateCountOnRange` read — explicit
                // verified count.
                count: Some(count),
            }]);
        }

        // Distinct mode (with or without In on prefix): walk and
        // emit per-`(in_key, key)` entries. Bounded by the request's
        // `limit` clause — the dispatcher already clamped that to
        // `max_query_limit`, so this walk is O(limit × log n) and
        // can't blow past the operator's DoS budget.
        //
        // Builds a single path query via the unified
        // `distinct_count_path_query` builder. For an Equal-only
        // prefix this collapses to a flat range-only query at the
        // terminator's property-name subtree; for an In-on-prefix
        // it becomes a compound query with one outer `Key` per In
        // value (sorted lex-ascending by the builder) plus a
        // `subquery_path`/`subquery` descending to the terminator's
        // range item. The builder pushes the caller's `limit` and
        // `order_by_ascending` directly into grovedb so the walk
        // stops at `limit` elements in the requested direction —
        // no Rust-side sort/reverse/truncate needed.
        let (path_query_limit, left_to_right) =
            (options.limit.map(|l| l as u16), options.order_by_ascending);
        let path_query =
            self.distinct_count_path_query(path_query_limit, left_to_right, platform_version)?;
        let base_path_len = path_query.path.len();

        let mut drive_operations = vec![];
        let result = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            // PathKeyElementTrio so we can recover the In value from
            // the emitted element's full path (for compound queries
            // the In value sits at `path[base_path_len]` — the first
            // segment beyond the path query's `path`).
            QueryResultType::QueryPathKeyElementTrioResultType,
            &mut drive_operations,
            drive_version,
        );
        let elements = match result {
            Ok((elements, _)) => elements,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                ) =>
            {
                // No matching prefix path — distinct mode returns
                // an empty entry list. (Summed modes returned earlier
                // via the aggregate fast path, so the empty case is
                // distinct-only here.)
                return Ok(Vec::new());
            }
            Err(e) => return Err(e),
        };

        // Walk emitted `(path, key, element)` triples and build the
        // unmerged entry list. For compound (In-on-prefix) queries
        // the In value sits at `path[base_path_len]`; for flat
        // queries `path.len() == base_path_len` so `in_key` is
        // `None`. We DO NOT collapse multiple emitted entries with
        // the same `key` into one — that's the whole point of the
        // no-merge contract.
        let mut entries: Vec<SplitCountEntry> = Vec::new();
        for triple in elements.to_path_key_elements() {
            let (path, key, element) = triple;
            let count = element.count_value_or_default();
            if count == 0 {
                continue;
            }
            let in_key = if has_in_on_prefix && path.len() > base_path_len {
                Some(path[base_path_len].clone())
            } else {
                None
            };
            // Distinct-walk emits one entry per distinct value
            // with its verified count; always `Some(_)`.
            entries.push(SplitCountEntry {
                in_key,
                key,
                count: Some(count),
            });
        }

        // Distinct mode: grovedb already emitted entries in the
        // requested direction (controlled by `left_to_right`) and
        // truncated to the path-query limit, so we return the entry
        // list as-is. The In keys are lex-sorted by the builder
        // (see `distinct_count_path_query`), so the natural emit
        // order is `(in_key_lex_asc, key_lex_asc)` for ascending
        // and `(in_key_lex_desc, key_lex_desc)` for descending —
        // the documented order contract holds by construction.
        //
        // For pagination, callers narrow the range bound itself
        // (`color > <last-key>` for the next page) rather than
        // passing a cursor — see `RangeCountOptions::limit` doc.
        Ok(entries)
    }

    /// Generates a grovedb `AggregateCountOnRange` proof for a
    /// range-count query against a `range_countable` index. The returned
    /// proof bytes can be verified client-side via
    /// `GroveDb::verify_aggregate_count_query`, which yields
    /// `(root_hash, count)` — replacing the materialize-and-count proof
    /// path that capped at `u16::MAX` documents.
    ///
    /// Limitations vs. [`Self::execute_range_count_no_proof`]:
    /// - Returns ONLY the total count (a single number, no
    ///   per-distinct-value entries) — `AggregateCountOnRange` is a
    ///   single-aggregate primitive at the merk layer.
    /// - Requires the prefix to resolve to exactly one path. `In` on
    ///   prefix properties is not supported because grovedb's aggregate
    ///   primitive only lifts a single inner range.
    pub fn execute_aggregate_count_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query = self.aggregate_count_path_query(platform_version)?;
        // Destructure rather than `.unwrap()` — see the In fan-out branch
        // in `execute_range_count_no_proof` for rationale.
        let CostContext { value, cost: _ } = drive.grove.get_proved_path_query(
            &path_query,
            None,
            transaction,
            &drive_version.grove_version,
        );
        let proof = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }

    /// Generates a regular grovedb range proof against this count
    /// query's `range_countable` index — the distinct-counts-with-
    /// proof companion to [`Self::execute_aggregate_count_with_proof`].
    ///
    /// No new prover code: the leaf is a `ProvableCountTree` and
    /// merk's existing `prove_query` already emits `KVCount(key,
    /// value, count)` per matched in-range key (via
    /// `to_kv_count_node`). Each `count` is hash-bound to the merk
    /// root via `node_hash_with_count`, so the per-key correctness
    /// guarantee comes for free with the standard hash-chain check —
    /// the SDK-side
    /// [`drive_proof_verifier::verify_distinct_count_proof`] just
    /// pulls the counts out of the proof's op stream after the
    /// integrity check passes.
    ///
    /// Trade-off vs. the aggregate prove path:
    /// - Returns per-distinct-value counts (one `(key, count)` per
    ///   matched lot value), not just a single sum.
    /// - Proof size is O(distinct values matched), not O(log n) — so
    ///   ~1 `KVCount` op per matched key instead of subtree collapse
    ///   via `HashWithCount`. Still strictly smaller than
    ///   materialize-and-count, which would emit each underlying doc.
    pub fn execute_distinct_count_with_proof(
        &self,
        drive: &Drive,
        limit: u16,
        left_to_right: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query =
            self.distinct_count_path_query(Some(limit), left_to_right, platform_version)?;
        // Destructure rather than `.unwrap()` — see the In fan-out branch
        // in `execute_range_count_no_proof` for rationale.
        let CostContext { value, cost: _ } = drive.grove.get_proved_path_query(
            &path_query,
            None,
            transaction,
            &drive_version.grove_version,
        );
        let proof = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }

    /// Generates a grovedb **carrier** `AggregateCountOnRange` proof
    /// for `In + range` queries with `group_by = [in_field]`. The
    /// proof commits one aggregate count per resolved In branch
    /// via grovedb's carrier-subquery composition
    /// ([PR #663](https://github.com/dashpay/grovedb/pull/663)).
    ///
    /// Path query: see
    /// [`Self::carrier_aggregate_count_path_query`].
    ///
    /// Trade-off vs. the alternative
    /// [`Self::execute_distinct_count_with_proof`]
    /// (`GroupByCompound` shape):
    /// - **This** (carrier-ACOR): O(|In| · (log B + log C')) proof
    ///   bytes. One commit per merk-tree boundary node per In
    ///   branch — preserves the per-branch aggregate granularity
    ///   that `group_by = [in_field, range_field]` can't express
    ///   (the compound shape commits per-distinct-value-pair
    ///   entries).
    /// - **Alternative** (distinct compound): O(|In| · R · log C')
    ///   where R is distinct in-range values per branch. Carries
    ///   strictly more information (one `(in_key, range_key)`
    ///   pair per resolved doc) at substantially larger bytes.
    ///
    /// Verified client-side via
    /// [`grovedb::GroveDb::verify_aggregate_count_query_per_key`],
    /// which returns `(RootHash, Vec<(Vec<u8>, u64)>)`.
    ///
    /// # Arguments
    /// * `left_to_right` — proof-shaping bit. Threaded into the
    ///   outer `Query` via `Query::new_with_direction(left_to_right)`
    ///   on the inner carrier path query (see
    ///   [`Self::carrier_aggregate_count_path_query`]). `true` walks
    ///   the outer range ascending and emits the per-branch `u64`s
    ///   in lex-ascending key order; `false` walks descending and
    ///   emits them in lex-descending order. The serialized
    ///   `PathQuery` bytes differ between the two — the verifier
    ///   rebuilds the path query from `(query, limit, left_to_right)`
    ///   on its side, so the value passed here must match what the
    ///   caller will pass to
    ///   [`Self::verify_carrier_aggregate_count_proof`] or the
    ///   tenderdash root check fails.
    pub fn execute_carrier_aggregate_count_with_proof(
        &self,
        drive: &Drive,
        limit: Option<u16>,
        left_to_right: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query =
            self.carrier_aggregate_count_path_query(limit, left_to_right, platform_version)?;
        // Same destructure pattern as the sibling aggregate / distinct
        // executors. `get_proved_path_query` returns `CostContext<Result>`;
        // ignoring the cost field is the same pattern those use today.
        let CostContext { value, cost: _ } = drive.grove.get_proved_path_query(
            &path_query,
            None,
            transaction,
            &drive_version.grove_version,
        );
        let proof = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }
}
