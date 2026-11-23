use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    invalid, DocumentHistoryDriveQuery, DocumentHistoryDriveQueryExecutionResult,
    SerializedDocumentHistoryDriveQueryExecutionResult,
};
use crate::query::refuse_a_private_document_type;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::PathKeyOptionalElementTrio;
use grovedb::{Element, TransactionArg};

impl Drive {
    fn fetch_document_history_rows_v1(
        &self,
        query: &DocumentHistoryDriveQuery,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        version: &PlatformVersion,
    ) -> Result<Vec<PathKeyOptionalElementTrio>, Error> {
        refuse_a_private_document_type(document_type).map_err(Error::Query)?;
        if !document_type.documents_keep_history() {
            return Err(invalid("document type does not keep history"));
        }
        let path_query = query.construct_path_query(version)?;
        let (rows, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType,
            &mut vec![],
            &version.drive,
        )?;
        let mut rows = rows.to_path_key_elements();
        let mut primary =
            contract_document_type_path_vec(&query.contract_id, &query.document_type_name);
        primary.push(vec![0]);
        for (path, key, element) in &mut rows {
            if *path == primary
                && matches!(
                    element,
                    Element::Reference(..) | Element::ReferenceWithSumItem(..)
                )
            {
                *element = self
                    .grove
                    .get::<Vec<u8>, _>(
                        path.as_slice(),
                        key,
                        transaction,
                        &version.drive.grove_version,
                    )
                    .value?;
            }
        }
        Ok(rows
            .into_iter()
            .map(|(path, key, element)| (path, key, Some(element)))
            .collect())
    }

    /// Fetches a page of the per-type history tree with the document's
    /// current lifecycle metadata, keeping revision bodies serialized and
    /// validating them from their headers alone.
    pub(crate) fn fetch_document_history_serialized_v1(
        &self,
        query: &DocumentHistoryDriveQuery,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        version: &PlatformVersion,
    ) -> Result<SerializedDocumentHistoryDriveQueryExecutionResult, Error> {
        let rows =
            self.fetch_document_history_rows_v1(query, document_type, transaction, version)?;
        query.decode_result_serialized(rows, document_type, version)
    }

    /// Fetches a page of the per-type history tree with the document's
    /// current lifecycle metadata.
    pub(super) fn fetch_document_history_v1(
        &self,
        query: &DocumentHistoryDriveQuery,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        version: &PlatformVersion,
    ) -> Result<DocumentHistoryDriveQueryExecutionResult, Error> {
        let rows =
            self.fetch_document_history_rows_v1(query, document_type, transaction, version)?;
        query.decode_result(rows, document_type, None, version)
    }
}
