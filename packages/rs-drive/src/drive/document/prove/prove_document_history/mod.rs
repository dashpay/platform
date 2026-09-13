use crate::drive::document::history::{
    invalid, DocumentHistoryProofV1, DocumentHistoryQueryV1, DocumentHistoryV1,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves a page of a historical document's retained revisions with its
    /// lifecycle, through the method version the protocol selects.
    pub fn prove_document_history(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(DocumentHistoryV1, DocumentHistoryProofV1), Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .prove_document_history
        {
            1 => self.prove_document_history_v1_impl(
                query,
                document_type,
                transaction,
                platform_version,
            ),
            0 => Err(invalid(
                "document history is served from protocol version 14",
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_document_history".to_owned(),
                known_versions: vec![1],
                received: version,
            })),
        }
    }
}
