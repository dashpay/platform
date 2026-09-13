use crate::drive::document::history::{
    invalid, DocumentHistoryProofV1, DocumentHistoryQueryV1, DocumentHistoryV1,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proved page of a historical document's retained revisions
    /// and lifecycle, through the method version the protocol selects.
    pub fn verify_document_history(
        proof: &DocumentHistoryProofV1,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, DocumentHistoryV1), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document
            .verify_document_history
        {
            1 => {
                Self::verify_document_history_v1_impl(query, proof, document_type, platform_version)
            }
            0 => Err(invalid(
                "document history is served from protocol version 14",
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_document_history".to_owned(),
                known_versions: vec![1],
                received: version,
            })),
        }
    }
}
