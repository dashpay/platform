use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    invalid, DocumentHistoryDriveQuery, DocumentHistoryDriveQueryExecutionResult,
};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Fetches a page of the per-type history tree with the document's
    /// current lifecycle metadata.
    pub(super) fn fetch_document_history_v1(
        &self,
        query: &DocumentHistoryDriveQuery,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        version: &PlatformVersion,
    ) -> Result<DocumentHistoryDriveQueryExecutionResult, Error> {
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
        query.decode_result(
            rows.into_iter()
                .map(|(path, key, element)| (path, key, Some(element)))
                .collect(),
            document_type,
            version,
        )
    }
}
