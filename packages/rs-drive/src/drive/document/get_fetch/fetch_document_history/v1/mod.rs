use crate::drive::document::history::{invalid, DocumentHistoryQueryV1, DocumentHistoryV1};
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Fetches a page of the per-type history tree with the document's
    /// current lifecycle metadata.
    pub(super) fn fetch_document_history_v1(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        version: &PlatformVersion,
    ) -> Result<DocumentHistoryV1, Error> {
        self.fetch_document_history_with_presence_v1(query, document_type, transaction, version)
            .map(|(history, _)| history)
    }

    /// Fetches the page and reports whether the history tree exists, which
    /// decides whether a proof of the page carries an entries proof.
    pub(crate) fn fetch_document_history_with_presence_v1(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        version: &PlatformVersion,
    ) -> Result<(DocumentHistoryV1, bool), Error> {
        if !document_type.documents_keep_history() {
            return Err(invalid("document type does not keep history"));
        }
        let entries_query = query.entries_query(version)?;
        let metadata_query = query.metadata_query(version)?;
        let (metadata, _) = self.grove_get_raw_path_query(
            &metadata_query,
            transaction,
            grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType,
            &mut vec![],
            &version.drive,
        )?;
        let mut metadata = metadata.to_path_key_elements();
        for (path, key, element) in &mut metadata {
            if matches!(
                element,
                Element::Reference(..) | Element::ReferenceWithSumItem(..)
            ) {
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
        let (lifecycle, present) = query.lifecycle(
            metadata
                .into_iter()
                .map(|(path, key, element)| (path, key, Some(element)))
                .collect(),
            document_type,
            version,
        )?;
        let entries = if present {
            let (entries, _) = self.grove_get_raw_path_query(
                &entries_query,
                transaction,
                grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &version.drive,
            )?;
            query.decode_entries(entries.to_key_elements(), document_type, version)?
        } else {
            vec![]
        };
        Ok((
            DocumentHistoryV1 {
                entries,
                lifecycle: Some(lifecycle),
            },
            present,
        ))
    }
}
