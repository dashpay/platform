//! Range-count proof executor for
//! [`super::super::DocumentCountMode::RangeProof`] dispatch —
//! `prove = true` count queries with a range clause and empty
//! `group_by`. Uses grovedb's `AggregateCountOnRange` primitive
//! to emit a single u64 verified out of the proof.

use super::super::super::conditions::WhereClause;
use super::super::DriveDocumentCountQuery;
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Range-count proof via grovedb's `AggregateCountOnRange`.
    /// Returns proof bytes that the client verifies via
    /// `GroveDb::verify_aggregate_count_query`.
    pub fn execute_document_count_range_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "range count requires a `range_countable: true` index whose last \
                     property matches the range field"
                    .to_string(),
            ))
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
        };
        count_query.execute_aggregate_count_with_proof(self, transaction, platform_version)
    }
}
