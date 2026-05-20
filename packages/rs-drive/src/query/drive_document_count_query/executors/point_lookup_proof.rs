//! Point-lookup count proof executor for
//! [`super::super::DocumentCountMode::PointLookupProof`]
//! dispatch — `prove = true` count queries with no range clause
//! (Equal/`In` against a `countable: true` index, OR the
//! `documents_countable: true` fast path on empty where).

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
    /// Point-lookup count proof against a `countable: true`
    /// index for `prove = true` Equal/`In` count queries, OR —
    /// when the where clauses are empty and the document type
    /// has `documents_countable: true` — a proof of the type's
    /// primary-key CountTree (one merk path proof, O(log n)
    /// bytes).
    ///
    /// In both cases the SDK-side verifier extracts each verified
    /// CountTree element's `count_value` directly, no document
    /// materialization.
    ///
    /// Mirrors the no-proof `Total` / `PerInValue` modes'
    /// rejection contract: if no `countable: true` index exactly
    /// covers the where clauses (and the documents_countable
    /// fast path doesn't apply), rejects with
    /// `WhereClauseOnNonIndexedProperty`. Same contract on both
    /// prove and no-proof paths — no silent fallback.
    pub fn execute_document_count_point_lookup_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;

        // Fast path: unfiltered prove count on a
        // `documents_countable: true` document type proves the
        // primary-key CountTree element directly. Same path-query
        // shape as the index-based case, just rooted at
        // `[..., doctype]` instead of inside an index.
        if where_clauses.is_empty() && document_type.documents_countable() {
            let path_query = DriveDocumentCountQuery::primary_key_count_tree_path_query(
                contract_id,
                &document_type_name,
            );
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

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "prove count requires a `countable: true` index whose properties \
                 exactly match the where clause fields, or `documentsCountable: \
                 true` on the document type for unfiltered total counts — same \
                 requirement as the no-proof path"
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
        count_query.execute_point_lookup_count_with_proof(self, transaction, platform_version)
    }
}
