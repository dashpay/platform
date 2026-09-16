use crate::drive::document::history::{DocumentHistoryProofV1, DocumentHistoryQueryV1};
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Produces independent pagination and metadata proofs of the per-type
    /// history tree from the same state.
    pub(super) fn prove_document_history_v1(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        version: &PlatformVersion,
    ) -> Result<DocumentHistoryProofV1, Error> {
        let (_, present) = self.fetch_document_history_with_presence_v1(
            query,
            document_type,
            transaction,
            version,
        )?;
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
        Ok(DocumentHistoryProofV1 {
            entries_proof,
            metadata_proof,
        })
    }
}
