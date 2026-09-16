//! Reading and proving a history page (server side).

use super::{invalid, DocumentHistoryProofV1, DocumentHistoryQueryV1, DocumentHistoryV1};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::Element;

#[cfg(feature = "server")]
impl Drive {
    /// Fetches a composite-keyed page and its current lifecycle metadata.
    pub(crate) fn fetch_document_history_v1_impl(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: grovedb::TransactionArg,
        version: &PlatformVersion,
    ) -> Result<DocumentHistoryV1, Error> {
        self.fetch_document_history_with_presence(query, document_type, transaction, version)
            .map(|(history, _)| history)
    }

    fn fetch_document_history_with_presence(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: grovedb::TransactionArg,
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
        Ok((DocumentHistoryV1 { entries, lifecycle }, present))
    }

    /// Produces independent pagination and metadata proofs from the same state.
    pub(crate) fn prove_document_history_v1_impl(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: grovedb::TransactionArg,
        version: &PlatformVersion,
    ) -> Result<(DocumentHistoryV1, DocumentHistoryProofV1), Error> {
        let (history, present) =
            self.fetch_document_history_with_presence(query, document_type, transaction, version)?;
        let metadata_proof = self.grove_get_proved_path_query(
            &query.metadata_query(version)?,
            transaction,
            &mut vec![],
            &version.drive,
        )?;
        let entries_proof = if !present {
            None
        } else {
            Some(self.grove_get_proved_path_query(
                &query.entries_query(version)?,
                transaction,
                &mut vec![],
                &version.drive,
            )?)
        };
        Ok((
            history,
            DocumentHistoryProofV1 {
                entries_proof,
                metadata_proof,
            },
        ))
    }
}

#[cfg(feature = "server")]
impl Drive {
    /// Fetches a page of a historical document's history with its lifecycle.
    pub fn fetch_document_history_v1(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: grovedb::TransactionArg,
        version: &PlatformVersion,
    ) -> Result<DocumentHistoryV1, Error> {
        match version.drive.methods.document.query.fetch_document_history {
            1 => self.fetch_document_history_v1_impl(query, document_type, transaction, version),
            0 => Err(invalid(
                "document history is served from protocol version 14",
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_history_v1".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Proves a page of a historical document's history with its lifecycle.
    pub fn prove_document_history_v1(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: grovedb::TransactionArg,
        version: &PlatformVersion,
    ) -> Result<(DocumentHistoryV1, DocumentHistoryProofV1), Error> {
        match version.drive.methods.document.query.prove_document_history {
            1 => self.prove_document_history_v1_impl(query, document_type, transaction, version),
            0 => Err(invalid(
                "document history is served from protocol version 14",
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_document_history_v1".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
