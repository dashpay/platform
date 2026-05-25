//! Joint count-and-sum `RangeNoProof` executor for
//! [`DocumentSumMode::RangeNoProof`] dispatch on the AVG no-prove path.
//!
//! Two structurally different paths share this executor:
//!
//! ## Aggregate shapes (`Aggregate + range`, `GroupByIn + range`)
//!
//! One `grove.query_aggregate_count_and_sum` call against the index's
//! `aggregate_count_and_sum_path_query` — a single merk-internal
//! `(u128, i128)` accumulator (narrowed to `(u64, i64)` at the
//! grovedb entry point) yielding both metrics in one O(log n)
//! traversal. Bounded **regardless of how many documents the range
//! matches**, so the public DAPI surface stays closed against
//! amplification.
//!
//! For compound `(In + range)` (with `In` on a prefix property) the
//! aggregate primitive can't fork through an `In`; the executor
//! per-In fans out (≤100 branches per the `In::in_values()` validator
//! cap) and issues one combined accumulator call per branch under a
//! shared read transaction. Worst-case 100 merk-internal reads per
//! request, again independent of matched-document count.
//!
//! ## Distinct shapes (`GroupByRange + range`, `GroupByCompound + range`)
//!
//! Walk PCPS terminator elements via
//! [`DriveDocumentSumQuery::distinct_sum_path_query`] in one
//! `grove_get_raw_path_query` call — the same shape sum's distinct
//! branch uses — and decode each via
//! [`grovedb::Element::count_sum_value_or_default`] to populate
//! [`AverageEntry`] with both `count` and `sum`. One walk yields both
//! axes per visited element instead of two parallel walks zipped
//! post-hoc. The distinct walk is bounded by the request's `limit`
//! (default falls back to `drive_config.default_query_limit`,
//! explicit limits are clamped to `drive_config.max_query_limit`) so
//! the public-endpoint amplification surface stays closed on this
//! path too.

use super::super::super::drive_document_average_query::{AverageEntry, DocumentAverageResponse};
use super::super::super::drive_document_sum_query::index_picker::find_range_summable_index_for_where_clauses;
use super::super::super::drive_document_sum_query::DriveDocumentSumQuery;
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
    /// `GroupByCompound`) depending on `return_distinct`. The dispatcher
    /// sets that flag based on the request's
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
        resolved_time_range_fields: &[String],
        sum_property: String,
        return_distinct: bool,
        left_to_right: bool,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            &sum_property,
            resolved_time_range_fields,
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

        if !return_distinct {
            // Aggregate shape: one combined merk-internal accumulator
            // call (`query_aggregate_count_and_sum`) yielding
            // `(u64, i64)` in O(log n) — strictly bounded regardless
            // of how many documents the range matches. Compound
            // `In + range` per-In fans out to one accumulator call
            // per branch under a shared read transaction (see
            // `aggregate_range_count_and_sum` below).
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
            sum_query.distinct_sum_path_query(limit, left_to_right, platform_version)?;
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
            // Drop fully-empty rows only. Sum's standalone distinct
            // executor drops on `sum == 0` regardless of count, but
            // that's overly aggressive for AVG: a row with
            // `count = N, sum = 0` is informative — it means the
            // group has N documents whose averageable values sum to
            // zero (e.g. all zero, or signed values that cancel). The
            // joint executor preserves such rows and only drops
            // grovedb-absent rows that decode as `(0, 0)`. Diverges
            // intentionally from sum's drop predicate.
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
    /// combined `query_aggregate_count_and_sum` accumulator. One call
    /// for the flat shape; compound `In + range` per-In fans out and
    /// sums per-branch totals (one accumulator call per branch). All
    /// branches share a read transaction (opened internally when the
    /// caller didn't supply one) so per-In sub-reads see the same
    /// grovedb snapshot.
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

        // Open a shared read transaction across per-In branches in
        // the compound shape so each branch's accumulator call sees
        // the same grovedb snapshot. The flat path issues a single
        // read and gets atomicity for free; the transaction is
        // harmless there. Read-only; dropped without commit at scope
        // end.
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
    /// `query_aggregate_count_and_sum` call against the PCPS path
    /// query — a single O(log n) merk-internal accumulator yielding
    /// both metrics from one traversal.
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
        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
            sum_property,
        };
        let path_query = sum_query.aggregate_count_and_sum_path_query(platform_version)?;
        let CostContext { value, cost: _ } = self.grove.query_aggregate_count_and_sum(
            &path_query,
            transaction,
            &drive_version.grove_version,
        );
        value.map_err(|e| Error::GroveDB(Box::new(e)))
    }
}
