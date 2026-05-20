//! Joint count-and-sum `RangeNoProof` executor for
//! [`DocumentSumMode::RangeNoProof`] dispatch on the AVG no-prove path.
//!
//! Two structurally different paths share this executor:
//!
//! ## Aggregate shapes (`Aggregate + range`, `GroupByIn + range`)
//!
//! Use grovedb's merk-internal aggregate primitives —
//! `query_aggregate_count` against the index's
//! `aggregate_count_path_query` AND `query_aggregate_sum` against the
//! same index's `aggregate_sum_path_query`. Two O(log n) merk-internal
//! reads, bounded **regardless of how many documents the range
//! matches**.
//!
//! grovedb's pinned rev does not yet expose a combined no-proof
//! `query_aggregate_count_and_sum` primitive (its proof-side analog
//! `AggregateCountAndSumOnRange` exists and is what the AVG prove path
//! uses). A future optional grovedb optimization could collapse these
//! into a single merk-internal accumulator; #3687 captures the
//! follow-up. Crucially, the no-proof path here still uses the
//! engine-side bounded accumulators — it does NOT walk every matched
//! PCPS element to fold `(count, sum)` in Rust. That walk shape would
//! turn a public DAPI endpoint into a request-amplification surface
//! (O(matching range keys) per request); the bounded accumulators close
//! that surface and match the worst-case cost the pre-#3687 composed
//! count + sum dispatchers had.
//!
//! For compound `(In + range)` (with `In` on a prefix property) the
//! aggregate primitive can't fork through an `In`; the executor
//! per-In fans out (≤100 branches per the `In::in_values()` validator
//! cap) and issues one count + one sum aggregate call per branch under
//! a shared read transaction. Worst-case 200 merk-internal reads per
//! request, again independent of matched-document count.
//!
//! ## Distinct shapes (`GroupByRange + range`, `GroupByCompound + range`)
//!
//! Walk PCPS terminator elements via
//! [`DriveDocumentSumQuery::distinct_sum_path_query`] in one
//! `grove_get_raw_path_query` call — the same shape sum's distinct
//! branch uses — and decode each via
//! [`grovedb::Element::count_sum_value_or_default`] to populate
//! [`AverageEntry`] with both `count` and `sum`. **This** is where the
//! single-walk win lives: one walk yields both axes per visited
//! element instead of two parallel walks zipped post-hoc. The
//! distinct walk is bounded by the request's `limit` (default falls
//! back to `drive_config.default_query_limit`, explicit limits are
//! clamped to `drive_config.max_query_limit`) so the public-endpoint
//! amplification surface stays closed on this path too.

use super::super::super::drive_document_average_query::{AverageEntry, DocumentAverageResponse};
use super::super::super::drive_document_count_query::DriveDocumentCountQuery;
use super::super::super::drive_document_sum_query::index_picker::find_range_summable_index_for_where_clauses;
use super::super::super::drive_document_sum_query::{DriveDocumentSumQuery, RangeSumOptions};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{WhereClause, WhereOperator};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;
use grovedb_costs::CostContext;

impl Drive {
    /// Range-aware joint count-and-sum walk against a
    /// `rangeSummable + rangeCountable` (PCPS-eligible) index.
    ///
    /// Returns either a one-pair [`DocumentAverageResponse::Aggregate`]
    /// (flat / compound aggregate shapes) or per-distinct-value
    /// [`DocumentAverageResponse::Entries`] (`GroupByRange` /
    /// `GroupByCompound`) depending on
    /// `options.return_distinct_sums_in_range`. The dispatcher sets
    /// that flag based on the request's
    /// [`super::super::super::drive_document_average_query::AverageMode`].
    ///
    /// `limit` applies only to the distinct branch; the aggregate
    /// branches return a single collapsed pair regardless. See the
    /// module docstring for the per-shape cost contract.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_and_sum_range_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        options: RangeSumOptions,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            &sum_property,
        )
        .filter(|idx| idx.range_countable)
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "average range query requires an index that declares BOTH \
                 `rangeSummable: true` AND `rangeCountable: true` (a \
                 `rangeAverageable: true` index is the shorthand) whose last \
                 property matches the range field"
                    .to_string(),
            ))
        })?;

        let drive_version = &platform_version.drive;
        let has_in_on_prefix = where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);

        if !options.return_distinct_sums_in_range {
            // Aggregate shape: use grovedb's merk-internal bounded
            // accumulators for `(count, sum)`. Two calls because the
            // combined no-proof primitive doesn't exist yet, but each
            // is O(log n) — strictly bounded regardless of how many
            // documents the range matches. This is the same cost class
            // the pre-#3687 composed count + sum dispatchers had.
            return self.aggregate_range_count_and_sum(
                contract_id,
                document_type,
                document_type_name,
                index,
                where_clauses,
                sum_property,
                has_in_on_prefix,
                transaction,
                platform_version,
            );
        }

        // Distinct shape: walk PCPS terminator elements via the
        // distinct path query, bounded by the caller's `limit`.
        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
            sum_property,
        };
        let path_query =
            sum_query.distinct_sum_path_query(limit, options.left_to_right, platform_version)?;
        let base_path_len = path_query.path.len();

        let mut drive_operations = vec![];
        let result = self.grove_get_raw_path_query(
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
                return Ok(DocumentAverageResponse::Entries(Vec::new()));
            }
            Err(e) => return Err(e),
        };

        let mut entries: Vec<AverageEntry> = Vec::new();
        for triple in elements.to_path_key_elements() {
            let (path, key, element) = triple;
            let (count, sum) = element.count_sum_value_or_default();
            // Drop empty groups: matches sum's distinct contract
            // (which drops `sum == 0`). For AVG an empty group has no
            // averageable signal AND no count signal.
            if count == 0 && sum == 0 {
                continue;
            }
            let in_key = if has_in_on_prefix && path.len() > base_path_len {
                Some(path[base_path_len].clone())
            } else {
                None
            };
            entries.push(AverageEntry {
                in_key,
                key,
                count: Some(count),
                sum: Some(sum),
            });
        }

        Ok(DocumentAverageResponse::Entries(entries))
    }

    /// Aggregate-range branch: returns `(count, sum)` via grovedb's
    /// merk-internal bounded accumulators. Two calls per branch
    /// (count + sum); compound `In + range` per-In fans out and sums
    /// per-branch totals.
    ///
    /// All branches share a read transaction (opened internally when
    /// the caller didn't supply one) so the count and sum reads see
    /// the same grovedb snapshot — same atomicity contract the
    /// pre-#3687 dispatcher implemented across its two sub-dispatches.
    #[allow(clippy::too_many_arguments)]
    fn aggregate_range_count_and_sum(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        index: &dpp::data_contract::document_type::Index,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        has_in_on_prefix: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        let drive_version = &platform_version.drive;

        // Open a shared read transaction across the count and sum
        // accumulator calls (and across per-In branches in the compound
        // shape) so they see one snapshot. Read-only; dropped without
        // commit at scope end.
        let local_tx;
        let effective_transaction: TransactionArg = if transaction.is_some() {
            transaction
        } else {
            local_tx = self.grove.start_transaction();
            Some(&local_tx)
        };

        if !has_in_on_prefix {
            let (count, sum) = self.flat_aggregate_count_and_sum(
                contract_id,
                document_type,
                document_type_name,
                index,
                where_clauses,
                sum_property,
                effective_transaction,
                drive_version,
                platform_version,
            )?;
            return Ok(DocumentAverageResponse::Aggregate { count, sum });
        }

        // Compound: per-In fan-out with the In replaced by Equal-per-
        // value. Exactly one In clause allowed — sum's existing
        // executor documents the silent-drop bug this guards.
        let in_clauses: Vec<&WhereClause> = where_clauses
            .iter()
            .filter(|wc| wc.operator == WhereOperator::In)
            .collect();
        if in_clauses.len() != 1 {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "compound range count-and-sum requires exactly one `in` clause",
                ),
            ));
        }
        let in_clause = in_clauses[0];
        let in_values = in_clause.in_values().into_data_with_error()??;
        let other_clauses: Vec<WhereClause> = where_clauses
            .iter()
            .filter(|wc| wc.operator != WhereOperator::In)
            .cloned()
            .collect();

        let mut count_total: u64 = 0;
        let mut sum_total: i64 = 0;
        let mut seen_keys: std::collections::BTreeSet<Vec<u8>> = std::collections::BTreeSet::new();
        for value in in_values.iter() {
            // Dedupe by canonical serialized bytes — DPP `in_values()`
            // already rejects raw-Value duplicates, but defense-in-
            // depth for future Value variants that serialize identically.
            let key_bytes = document_type.serialize_value_for_key(
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

            let (branch_count, branch_sum) = self.flat_aggregate_count_and_sum(
                contract_id,
                document_type,
                document_type_name.clone(),
                index,
                clauses_for_value,
                sum_property.clone(),
                effective_transaction,
                drive_version,
                platform_version,
            )?;
            // `checked_add` rather than `saturating_add` so an
            // overflow fails deterministically on both axes —
            // matches the pattern in the sibling `total` /
            // `per_in_value` executors.
            count_total = count_total.checked_add(branch_count).ok_or_else(|| {
                Error::Query(QuerySyntaxError::Unsupported(
                    "compound In-on-prefix range count-and-sum overflowed u64 on the count \
                     axis when summing per-In aggregates. Narrow the query (smaller In set \
                     or narrower range) or use multiple queries and combine client-side."
                        .to_string(),
                ))
            })?;
            sum_total = sum_total.checked_add(branch_sum).ok_or_else(|| {
                Error::Query(QuerySyntaxError::Unsupported(
                    "compound In-on-prefix range count-and-sum overflowed i64 on the sum \
                     axis when summing per-In aggregates. Narrow the query (smaller In set \
                     or narrower range) or use multiple queries and combine client-side."
                        .to_string(),
                ))
            })?;
        }

        Ok(DocumentAverageResponse::Aggregate {
            count: count_total,
            sum: sum_total,
        })
    }

    /// Flat (no In on prefix) aggregate count + sum: one
    /// `query_aggregate_count` call against the count path query, one
    /// `query_aggregate_sum` call against the sum path query, both
    /// O(log n) at the merk layer. Shared `transaction` enforces
    /// snapshot consistency across the two reads.
    #[allow(clippy::too_many_arguments)]
    fn flat_aggregate_count_and_sum(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        index: &dpp::data_contract::document_type::Index,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        transaction: TransactionArg,
        drive_version: &dpp::version::drive_versions::DriveVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(u64, i64), Error> {
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name: document_type_name.clone(),
            index,
            where_clauses: where_clauses.clone(),
        };
        let count_path_query = count_query.aggregate_count_path_query(platform_version)?;
        let CostContext { value, cost: _ } = self.grove.query_aggregate_count(
            &count_path_query,
            transaction,
            &drive_version.grove_version,
        );
        let count = value.map_err(|e| Error::GroveDB(Box::new(e)))?;

        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
            sum_property,
        };
        let sum_path_query = sum_query.aggregate_sum_path_query(platform_version)?;
        let CostContext { value, cost: _ } = self.grove.query_aggregate_sum(
            &sum_path_query,
            transaction,
            &drive_version.grove_version,
        );
        let sum = value.map_err(|e| Error::GroveDB(Box::new(e)))?;

        Ok((count, sum))
    }
}
