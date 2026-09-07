//! Range-sum prove executor.
//! Mirror of count's `executors/range_proof.rs`. Builds an
//! `AggregateSumOnRange` path query (grovedb PR 670) and returns proof
//! bytes verifiable via `GroveDb::verify_aggregate_sum_query`.

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
    /// Range-sum proof via grovedb's `AggregateSumOnRange` primitive
    /// (PR 670). Returns proof bytes verified via
    /// `GroveDb::verify_aggregate_sum_query`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_sum_range_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        resolved_time_ranges: &[ResolvedTimeRange],
        sum_property: String,
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
                "range sum requires a `rangeSummable: true` index whose last \
                 property matches the range field"
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
        sum_query.execute_aggregate_sum_with_proof(self, transaction, platform_version)
    }
}
