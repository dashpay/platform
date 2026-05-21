//! Per-distinct-key range-sum prove executor.
//! Mirror of count's `executors/range_distinct_proof.rs`. Returns
//! one `SumEntry` per distinct in-range value via `KVSum` ops.

use super::super::index_picker::find_range_summable_index_for_where_clauses;
use super::super::DriveDocumentSumQuery;
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::WhereClause;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Distinct-sums-with-proof companion to
    /// [`Self::execute_document_sum_range_proof`]. Returns proof bytes
    /// the client verifies via the per-distinct-sum verifier
    /// (pending), yielding `BTreeMap<Vec<u8>, i64>` per distinct
    /// value.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_sum_range_distinct_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        limit: u16,
        left_to_right: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            &sum_property,
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
        sum_query.execute_distinct_sum_with_proof(
            self,
            limit,
            left_to_right,
            transaction,
            platform_version,
        )
    }
}
