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

use super::super::conditions::WhereOperator;
use super::{DriveDocumentCountQuery, SplitCountEntry};
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;

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
    /// index. Walks children of the property-name `ProvableCountTree` at
    /// path `[contract_doc, doctype, prefix..., range_prop_name]` whose
    /// keys lie within the range. Each child is a `CountTree` whose
    /// `count_value_or_default()` is the document count at that property
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
    /// `In` on the prefix forks the walk into one path per (deduped)
    /// `In` value. Each emitted entry carries its `in_key` (the In
    /// value for that fork) alongside the `key` (the terminator
    /// value). Cross-fork aggregation is intentionally NOT performed
    /// server-side — callers reduce by `key` client-side if they
    /// want a flat histogram. See the book chapter ("Range Modes")
    /// for rationale.
    ///
    /// When `options.distinct = false`, returns a single entry with
    /// `in_key = None`, empty `key`, and `count` equal to the sum of
    /// all matched per-value counts (the natural reduction). When
    /// `options.distinct = true`, returns one entry per emitted
    /// `(in_key, key)` pair, after applying `order_by_ascending`
    /// and `limit` over the lexicographic `(in_key, key)` tuple.
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

        // Flat (no-In) summed mode has a dedicated O(log n) fast
        // path via grovedb's no-proof `AggregateCountOnRange`
        // execution (`GroveDb::query_aggregate_count`). It walks the
        // merk tree's boundary nodes using each node's stored
        // aggregate count to short-circuit fully-inside/outside
        // subtrees, returning the count directly without
        // materializing any child elements. Compound (`In + range`)
        // summed mode can't use this primitive because
        // `AggregateCountOnRange` is a single-range merk operation
        // that doesn't fork over outer `Key` items — for that case
        // we fall through to the walk-and-sum path below.
        if !options.distinct && !has_in_on_prefix {
            let path_query = self.aggregate_count_path_query(platform_version)?;
            let count = drive
                .grove
                .query_aggregate_count(&path_query, transaction, &drive_version.grove_version)
                .unwrap()
                .map_err(|e| Error::GroveDB(Box::new(e)))?;
            return Ok(vec![SplitCountEntry {
                in_key: None,
                key: Vec::new(),
                count,
            }]);
        }

        // Walk-and-sum / walk-and-emit path. Used by:
        // - Compound summed mode (the aggregate primitive can't fork
        //   over `In`, so we materialize each `(in_key, key)` entry
        //   and sum in Rust).
        // - Distinct mode (caller wants per-`(in_key, key)` entries,
        //   not a single sum).
        //
        // Builds a single path query via the unified
        // `distinct_count_path_query` builder. For an Equal-only
        // prefix this collapses to a flat range-only query at the
        // terminator's property-name subtree; for an In-on-prefix
        // it becomes a compound query with one outer `Key` per In
        // value (sorted lex-ascending by the builder) plus a
        // `subquery_path`/`subquery` descending to the terminator's
        // range item.
        //
        // Limit and direction handling differs by mode:
        // - **Compound summed mode** needs every emitted element to
        //   compute the aggregate, so the path-query limit stays
        //   `None` and direction is the canonical ascending. The
        //   per-query DoS bound is the index size itself, bounded
        //   by the contract author's index choice.
        // - **Distinct mode** pushes the caller's `limit` and
        //   `order_by_ascending` directly into grovedb so the walk
        //   stops at `limit` elements in the requested direction.
        //   Per-query work is then O(limit × log n) instead of
        //   O(index size), and no Rust-side sort/reverse/truncate
        //   is needed.
        let (path_query_limit, left_to_right) = if options.distinct {
            (options.limit.map(|l| l as u16), options.order_by_ascending)
        } else {
            (None, true)
        };
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
                // No matching prefix path — return zero/empty per
                // mode below.
                return Ok(if !options.distinct {
                    vec![SplitCountEntry {
                        in_key: None,
                        key: Vec::new(),
                        count: 0,
                    }]
                } else {
                    Vec::new()
                });
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
            entries.push(SplitCountEntry { in_key, key, count });
        }

        if !options.distinct {
            // Summed mode: sum across all emitted entries (across
            // both forks and per-terminator-value sub-counts).
            // Returns a single `in_key: None, key: empty` entry with
            // the aggregate total — matches the wire-format
            // `aggregate_count` variant the abci handler will lift
            // it into.
            let total: u64 = entries.iter().map(|e| e.count).sum();
            return Ok(vec![SplitCountEntry {
                in_key: None,
                key: Vec::new(),
                count: total,
            }]);
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
        let proof = drive
            .grove
            .get_proved_path_query(&path_query, None, transaction, &drive_version.grove_version)
            .unwrap()
            .map_err(|e| Error::GroveDB(Box::new(e)))?;
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
        let proof = drive
            .grove
            .get_proved_path_query(&path_query, None, transaction, &drive_version.grove_version)
            .unwrap()
            .map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }
}
