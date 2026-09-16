mod v0;
mod v1;

use crate::drive::document::history::{DocumentHistoryProof, DocumentHistoryQueryV1};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves a page of a historical document's history in the layout the
    /// protocol version stores.
    pub fn prove_document_history(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentHistoryProof, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .prove_document_history
        {
            0 => {
                let (start_at_ms, limit) = query.legacy_read()?;
                self.prove_document_history_v0(
                    query.contract_id,
                    &query.document_type_name,
                    query.document_id,
                    transaction,
                    start_at_ms,
                    limit,
                    None,
                    platform_version,
                )
                .map(DocumentHistoryProof::V0)
            }
            1 => self
                .prove_document_history_v1(query, document_type, transaction, platform_version)
                .map(DocumentHistoryProof::V1),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_document_history".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
