//! Total-count executor for [`super::super::DocumentCountMode::Total`]
//! dispatch — `prove = false` count queries without a range clause.

use super::super::super::conditions::WhereClause;
use super::super::{DriveDocumentCountQuery, SplitCountEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Total count for the given where clauses against an exactly-
    /// covering countable index, OR — when the where clauses are
    /// empty and the document type has `documents_countable: true` —
    /// the type's primary-key CountTree (O(1) read at the doctype
    /// tree's root).
    ///
    /// Single summed entry with empty key.
    pub fn execute_document_count_total_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        use dpp::data_contract::document_type::accessors::{
            DocumentTypeV0Getters, DocumentTypeV2Getters,
        };

        // Fast path: unfiltered total count on a `documents_countable:
        // true` document type reads the primary-key CountTree directly
        // (O(1)). No index needed — the doctype tree itself carries
        // the count.
        if where_clauses.is_empty() && document_type.documents_countable() {
            let count = self.read_primary_key_count_tree(
                &contract_id,
                &document_type_name,
                transaction,
                platform_version,
            )?;
            return Ok(vec![SplitCountEntry {
                in_key: None,
                key: vec![],
                // `documents_countable` fast path: we read the
                // CountTree directly and got an explicit count, so
                // this is a verified `Some(_)` (possibly `Some(0)`
                // for an empty doctype).
                count: Some(count),
            }]);
        }

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "count query requires a `countable: true` index whose properties \
                     exactly match the where clause fields, or `documentsCountable: \
                     true` on the document type for unfiltered total counts"
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
        count_query.execute_no_proof(self, transaction, platform_version)
    }

    /// Reads the document-type primary-key tree's `CountTree` element
    /// (`[contract_doc, contract_id, [1], doctype, 0]`) and returns
    /// `count_value_or_default()`. Used by the `documents_countable:
    /// true` fast path on the total-count flow.
    ///
    /// Returns 0 when the element doesn't exist (e.g. fresh contract
    /// with no documents inserted). Caller is responsible for ensuring
    /// `documents_countable` is set on the document type before
    /// calling — without it the element at `[..., doctype, 0]` is a
    /// regular `NormalTree` and `count_value_or_default()` returns 0
    /// regardless of how many documents the type actually has.
    pub(super) fn read_primary_key_count_tree(
        &self,
        contract_id: &[u8; 32],
        document_type_name: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let drive_version = &platform_version.drive;
        let path = [
            &[crate::drive::RootTree::DataContractDocuments as u8] as &[u8],
            contract_id,
            &[1u8],
            document_type_name.as_bytes(),
        ];
        let mut drive_operations = vec![];
        let element = self.grove_get_raw_optional(
            grovedb_path::SubtreePath::from(path.as_slice()),
            &[0],
            crate::util::grove_operations::DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;
        Ok(element.map_or(0, |e| e.count_value_or_default()))
    }
}
