//! Point-lookup prove executor for sum queries.
//! Mirror of count's `executors/point_lookup_proof.rs`.

use super::super::index_picker::find_summable_index_for_where_clauses;
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
    /// Point-lookup sum proof against a `summable: "<x>"` index for
    /// `prove = true` Equal/`In` sum queries, OR — when the where
    /// clauses are empty and the document type has
    /// `documents_summable: Some(matching_property)` — a proof of the
    /// type's primary-key SumTree.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_sum_point_lookup_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        resolved_time_range_fields: &[String],
        sum_property: String,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;

        // Fast path: unfiltered prove sum on a `documents_summable:
        // Some(matching_property)` document type proves the
        // primary-key SumTree element directly.
        if where_clauses.is_empty()
            && document_type
                .documents_summable()
                .map(|p| p == sum_property)
                .unwrap_or(false)
        {
            let path_query =
                DriveDocumentSumQuery::primary_key_sum_path_query(contract_id, &document_type_name);
            let proof = self
                .grove
                .get_proved_path_query(
                    &path_query,
                    None,
                    transaction,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .map_err(|e| Error::GroveDB(Box::new(e)))?;
            return Ok(proof);
        }

        let index = find_summable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            &sum_property,
            resolved_time_range_fields,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "prove sum requires a `summable: \"<prop>\"` index whose properties \
                 exactly match the where clause fields and whose summed property \
                 matches the request's `sum_property`, or `documentsSummable: \
                 \"<prop>\"` on the document type for unfiltered total sums — same \
                 requirement as the no-proof path"
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
        sum_query.execute_point_lookup_sum_with_proof(self, transaction, platform_version)
    }
}
