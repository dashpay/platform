//! Range execution paths for the sum query. Parallels count's
//! `execute_range_count.rs`.
//!
//! - [`DriveDocumentSumQuery::execute_range_sum_no_proof`] — Rust-side
//!   walk via `query_aggregate_sum` (or per-In fan-out for compound
//!   shapes), returning a single `Aggregate` entry or per-(in_key, key)
//!   distinct entries without a proof.
//! - [`DriveDocumentSumQuery::execute_aggregate_sum_with_proof`] —
//!   grovedb `AggregateSumOnRange` proof, returning a single i64
//!   verified out of the proof.
//! - [`DriveDocumentSumQuery::execute_distinct_sum_with_proof`] —
//!   regular range proof against the `ProvableSumTree`, returning
//!   per-key `KVSum` ops bound to the merk root.

use super::{DriveDocumentSumQuery, RangeSumOptions, SumEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{WhereClause, WhereOperator};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;
use grovedb_costs::CostContext;

impl DriveDocumentSumQuery<'_> {
    /// Range-aware sum walk against a `rangeSummable: true` index.
    ///
    /// Mirror of count's `execute_range_count_no_proof`. Routing:
    /// - **Flat summed** (no `In`, distinct=false): single
    ///   `query_aggregate_sum` call against the merk-level
    ///   `AggregateSumOnRange` primitive. O(log n).
    /// - **Compound summed** (`In` on prefix, distinct=false): per-In
    ///   fan-out — one `query_aggregate_sum` call per matched In
    ///   branch, summed in Rust.
    /// - **Distinct mode** (`distinct=true`): walks the unified
    ///   `distinct_sum_path_query` and emits one entry per matched
    ///   `(in_key, key)` pair. (Currently stubbed pending the
    ///   distinct-builder port.)
    pub fn execute_range_sum_no_proof(
        &self,
        drive: &Drive,
        options: &RangeSumOptions,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SumEntry>, Error> {
        let drive_version = &platform_version.drive;
        let has_in_on_prefix = self
            .where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);

        if !options.return_distinct_sums_in_range {
            if has_in_on_prefix {
                // Enforce exactly one `In` clause. Without this, a request
                // with multiple In filters would silently use only the
                // first and drop the rest, producing an over-broad total.
                let in_clauses: Vec<&WhereClause> = self
                    .where_clauses
                    .iter()
                    .filter(|wc| wc.operator == WhereOperator::In)
                    .collect();
                if in_clauses.len() != 1 {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "compound summed range sum path requires exactly one `in` clause",
                        ),
                    ));
                }
                let in_clause = in_clauses[0];
                let in_values = in_clause.in_values().into_data_with_error()??;
                let other_clauses: Vec<WhereClause> = self
                    .where_clauses
                    .iter()
                    .filter(|wc| wc.operator != WhereOperator::In)
                    .cloned()
                    .collect();

                let mut total: i64 = 0;
                let mut seen_keys: std::collections::BTreeSet<Vec<u8>> =
                    std::collections::BTreeSet::new();
                for value in in_values.iter() {
                    let key_bytes = self.document_type.serialize_value_for_key(
                        in_clause.field.as_str(),
                        value,
                        platform_version,
                    )?;
                    if !seen_keys.insert(key_bytes) {
                        continue;
                    }

                    let mut clauses_for_value = other_clauses.clone();
                    clauses_for_value.push(WhereClause {
                        field: in_clause.field.clone(),
                        operator: WhereOperator::Equal,
                        value: value.clone(),
                    });
                    let per_value_query = DriveDocumentSumQuery {
                        document_type: self.document_type,
                        contract_id: self.contract_id,
                        document_type_name: self.document_type_name.clone(),
                        index: self.index,
                        where_clauses: clauses_for_value,
                        sum_property: self.sum_property.clone(),
                    };
                    let path_query = per_value_query.aggregate_sum_path_query(platform_version)?;
                    let CostContext { value, cost: _ } = drive.grove.query_aggregate_sum(
                        &path_query,
                        transaction,
                        &drive_version.grove_version,
                    );
                    let sum = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
                    // Use `checked_add` rather than `saturating_add` so an
                    // overflowed aggregate fails deterministically instead
                    // of silently clamping at i64::MAX. The proof-side
                    // verifier sees the same overflow at the same point
                    // (the grovedb primitive itself returns i64), so
                    // refusing here keeps prover and verifier in sync
                    // on the rejection rather than letting the no-proof
                    // path return a value the proof path would reject.
                    total = total.checked_add(sum).ok_or_else(|| {
                        Error::Query(QuerySyntaxError::Unsupported(
                            "compound In-on-prefix range-sum overflowed i64 when summing \
                             per-In aggregates. Narrow the query (smaller In set, narrower \
                             range) or use multiple queries and combine client-side."
                                .to_string(),
                        ))
                    })?;
                }
                return Ok(vec![SumEntry {
                    in_key: None,
                    key: Vec::new(),
                    sum: Some(total),
                }]);
            }
            // Flat summed (no In on prefix): single aggregate read.
            let path_query = self.aggregate_sum_path_query(platform_version)?;
            let CostContext { value, cost: _ } = drive.grove.query_aggregate_sum(
                &path_query,
                transaction,
                &drive_version.grove_version,
            );
            let sum = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
            return Ok(vec![SumEntry {
                in_key: None,
                key: Vec::new(),
                sum: Some(sum),
            }]);
        }

        // Distinct mode. Mirror count's analog; currently relies on
        // `distinct_sum_path_query` which is stubbed (pending port).
        // Defer to the same builder so the error surfaces cleanly when
        // distinct mode is requested before the builder body lands.
        let (path_query_limit, left_to_right) = (None::<u16>, options.left_to_right);
        let path_query =
            self.distinct_sum_path_query(path_query_limit, left_to_right, platform_version)?;
        let base_path_len = path_query.path.len();

        let mut drive_operations = vec![];
        let result = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
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
                return Ok(Vec::new());
            }
            Err(e) => return Err(e),
        };

        let mut entries: Vec<SumEntry> = Vec::new();
        for triple in elements.to_path_key_elements() {
            let (path, key, element) = triple;
            let sum = element.sum_value_or_default();
            if sum == 0 {
                continue;
            }
            let in_key = if has_in_on_prefix && path.len() > base_path_len {
                Some(path[base_path_len].clone())
            } else {
                None
            };
            entries.push(SumEntry {
                in_key,
                key,
                sum: Some(sum),
            });
        }

        Ok(entries)
    }

    /// Generates a grovedb `AggregateSumOnRange` proof for a range-sum
    /// query against a `rangeSummable` index. Returned proof bytes
    /// verify via `GroveDb::verify_aggregate_sum_query` yielding
    /// `(root_hash, i64 sum)`.
    pub fn execute_aggregate_sum_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query = self.aggregate_sum_path_query(platform_version)?;
        let CostContext { value, cost: _ } = drive.grove.get_proved_path_query(
            &path_query,
            None,
            transaction,
            &drive_version.grove_version,
        );
        let proof = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }

    /// Per-distinct-key range-sum proof against this query's
    /// `rangeSummable` index. Mirror of count's
    /// `execute_distinct_count_with_proof`. Currently routes through
    /// `distinct_sum_path_query` which is stubbed (pending the
    /// ~280-line port from count); calls before that lands surface
    /// `Unsupported` cleanly.
    pub fn execute_distinct_sum_with_proof(
        &self,
        drive: &Drive,
        limit: u16,
        left_to_right: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query =
            self.distinct_sum_path_query(Some(limit), left_to_right, platform_version)?;
        let CostContext { value, cost: _ } = drive.grove.get_proved_path_query(
            &path_query,
            None,
            transaction,
            &drive_version.grove_version,
        );
        let proof = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }

    /// Generates a grovedb leaf-PCPS `AggregateCountAndSumOnRange`
    /// proof for a combined count + sum range query against an index
    /// that declares BOTH `rangeCountable: true` AND `rangeSummable:
    /// true`. Returned proof bytes verify via
    /// `GroveDb::verify_aggregate_count_and_sum_query` yielding
    /// `(root_hash, u64 count, i64 sum)` — the load-bearing primitive
    /// for the [average-index-examples chapter]
    /// (../../../../book/src/drive/average-index-examples.md)'s
    /// Query 5 ("Class Trend"). PCPS-only: the terminator's value tree
    /// MUST be a `ProvableCountProvableSumTree`; lighter
    /// (CountSumTree / ProvableCountSumTree / ProvableSumTree)
    /// terminators are rejected at the grovedb merk-gate.
    ///
    /// Leaf analog of
    /// [`Self::execute_carrier_aggregate_count_and_sum_with_proof`]:
    /// same primitive, no outer `In` fan-out — single
    /// `(count, sum)` per proof rather than per-In-key `(count, sum)`
    /// triples.
    pub fn execute_aggregate_count_and_sum_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query = self.aggregate_count_and_sum_path_query(platform_version)?;
        let CostContext { value, cost: _ } = drive.grove.get_proved_path_query(
            &path_query,
            None,
            transaction,
            &drive_version.grove_version,
        );
        let proof = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }

    /// Generates a grovedb **carrier** `AggregateSumOnRange` proof
    /// for `In + range` queries with `group_by = [in_field]` (and the
    /// `RangeAggregateCarrierProof` mode in general). Sum analog of
    /// count's
    /// [`crate::query::drive_document_count_query::DriveDocumentCountQuery::execute_carrier_aggregate_count_with_proof`].
    ///
    /// Builds the carrier `PathQuery` via
    /// [`Self::carrier_aggregate_sum_path_query`] and asks grovedb
    /// for a proof. The proof commits one aggregate sum per resolved
    /// In branch; verified client-side via
    /// `GroveDb::verify_aggregate_sum_query_per_key` (grovedb PR #670
    /// head `e98bab5f`), which returns `(RootHash, Vec<(Vec<u8>, i64)>)`.
    ///
    /// `left_to_right` and `limit` are byte-load-bearing — they are
    /// part of the `PathQuery` bytes the verifier rebuilds. See count's
    /// analog for the rationale.
    pub fn execute_carrier_aggregate_sum_with_proof(
        &self,
        drive: &Drive,
        limit: Option<u16>,
        left_to_right: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query =
            self.carrier_aggregate_sum_path_query(limit, left_to_right, platform_version)?;
        let CostContext { value, cost: _ } = drive.grove.get_proved_path_query(
            &path_query,
            None,
            transaction,
            &drive_version.grove_version,
        );
        let proof = value.map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }

    /// Combined PCPS carrier proof:
    /// `AggregateCountAndSumOnRange`-on-carrier. Sum-and-count analog
    /// of [`Self::execute_carrier_aggregate_sum_with_proof`]. Requires
    /// the chosen index to declare BOTH `rangeCountable: true` AND
    /// `rangeSummable: true` so the terminator's value tree is a
    /// `ProvableCountProvableSumTree`.
    ///
    /// Returns proof bytes the verifier maps to
    /// `Vec<(Vec<u8>, u64, i64)>` via
    /// `GroveDb::verify_aggregate_count_and_sum_query_per_key` (grovedb
    /// PR #670 head `e98bab5f`) — one `(in_key, count, sum)` triple
    /// per resolved In branch.
    pub fn execute_carrier_aggregate_count_and_sum_with_proof(
        &self,
        drive: &Drive,
        limit: Option<u16>,
        left_to_right: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query = self.carrier_aggregate_count_and_sum_path_query(
            limit,
            left_to_right,
            platform_version,
        )?;
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
