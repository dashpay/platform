use crate::drive::Drive;
use crate::error::Error;
use crate::query::document_history_drive_query::DocumentHistoryDriveQuery;
use crate::verify::document::DocumentHistoryProof;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Produces independent pagination and metadata proofs of the per-type
    /// history tree from the same state, carried as one proof envelope.
    pub(super) fn prove_document_history_v1(
        &self,
        query: &DocumentHistoryDriveQuery,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let (_, present) = self.fetch_document_history_with_presence_v1(
            query,
            document_type,
            transaction,
            version,
        )?;
        let metadata_proof = self.grove_get_proved_path_query(
            &query.metadata_path_query(version)?,
            transaction,
            &mut vec![],
            &version.drive,
        )?;
        let entries_proof = if !present {
            None
        } else {
            Some(self.grove_get_proved_path_query(
                &query.construct_path_query(version)?,
                transaction,
                &mut vec![],
                &version.drive,
            )?)
        };
        DocumentHistoryProof {
            metadata_proof,
            entries_proof,
        }
        .to_bytes()
    }
}
