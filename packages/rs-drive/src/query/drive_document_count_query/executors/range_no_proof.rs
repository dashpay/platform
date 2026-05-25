//! Range-count executor for
//! [`super::super::DocumentCountMode::RangeNoProof`] dispatch —
//! `prove = false` count queries with a range clause. Returns a
//! summed entry when `options.distinct = false` or per-distinct-
//! value entries when `options.distinct = true`.

use super::super::super::conditions::WhereClause;
use super::super::execute_range_count::RangeCountOptions;
use super::super::{DriveDocumentCountQuery, SplitCountEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Range-count walk against a `range_countable` index.
    /// Returns a summed entry or per-distinct-value entries
    /// depending on `options.distinct`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_range_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        resolved_time_range_fields: &[String],
        options: RangeCountOptions,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            resolved_time_range_fields,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "range count requires a `range_countable: true` index whose last \
                     property matches the range field, with all other clauses covering \
                     its prefix as `==` matches"
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
        count_query.execute_range_count_no_proof(self, &options, transaction, platform_version)
    }
}
