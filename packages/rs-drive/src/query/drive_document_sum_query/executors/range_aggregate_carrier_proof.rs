//! Carrier-aggregate sum prove executor. Mirror of count's
//! [`crate::query::drive_document_count_query::executors::range_aggregate_carrier_proof`].
//!
//! Routes a `(In OR outer Range) + inner range` sum request with
//! `group_by = [outer_field]` to the carrier `PathQuery` builder
//! ([`super::super::DriveDocumentSumQuery::carrier_aggregate_sum_path_query`])
//! and asks grovedb for a proof. The client verifies via
//! [`grovedb::GroveDb::verify_aggregate_sum_query_per_key`] (grovedb
//! PR #670 head `e98bab5f`), producing `Vec<(outer_key, i64)>` — same
//! per-key aggregate semantics as the no-proof per-In fan-out, just
//! verifiable.

use super::super::index_picker::find_range_summable_index_for_where_clauses;
use super::super::DriveDocumentSumQuery;
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::ResolvedTimeRange;
use crate::query::WhereClause;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Carrier-aggregate sum proof for `(In OR outer Range) + inner
    /// range` with `group_by = [outer_field]`. Returns proof bytes
    /// that the client verifies via
    /// [`grovedb::GroveDb::verify_aggregate_sum_query_per_key`].
    ///
    /// `limit` caps the outer walk for the Range-outer shape; for
    /// the In-outer shape the `|In|` array already bounds the result
    /// and `limit` is typically `None`.
    ///
    /// Sibling executors (`range_distinct_proof`, `range_no_proof`,
    /// `per_in_value`) use the same `#[allow]` here — the 9-arg
    /// boundary (contract id, doc type, doc type name, where clauses,
    /// sum_property, limit, left_to_right, transaction, platform
    /// version) is the established executor signature; refactoring it
    /// into a struct would just move the same fields one indirection
    /// away.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_sum_range_aggregate_carrier_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        resolved_time_ranges: &[ResolvedTimeRange],
        sum_property: String,
        limit: Option<u16>,
        left_to_right: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            &sum_property,
            resolved_time_ranges,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "carrier-aggregate sum requires a `rangeSummable: true` index whose first \
                 property carries the outer In or range clause and whose last property \
                 carries the inner ASOR range clause"
                    .to_string(),
            ))
        })?;
        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
            sum_property,
        };
        sum_query.execute_carrier_aggregate_sum_with_proof(
            self,
            limit,
            left_to_right,
            transaction,
            platform_version,
        )
    }
}
