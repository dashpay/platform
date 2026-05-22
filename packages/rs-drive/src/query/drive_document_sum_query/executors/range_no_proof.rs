//! Range-sum no-proof executor.
//! Mirror of count's `executors/range_no_proof.rs`.

use super::super::index_picker::find_range_summable_index_for_where_clauses;
use super::super::{DriveDocumentSumQuery, RangeSumOptions, SumEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::WhereClause;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Range-sum walk against a `rangeSummable: true` index. Returns
    /// a summed entry or per-distinct-value entries depending on
    /// `options.return_distinct_sums_in_range`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_sum_range_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        options: RangeSumOptions,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SumEntry>, Error> {
        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            &sum_property,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "range sum requires a `rangeSummable: true` index whose last \
                 property matches the range field, with all other clauses covering \
                 its prefix as `==` matches"
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
        sum_query.execute_range_sum_no_proof(self, &options, transaction, platform_version)
    }
}
