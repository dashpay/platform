mod v0;
mod v1;

use crate::drive::document::history::{
    invalid, DocumentHistoryProof, DocumentHistoryQueryV1, DocumentHistoryV1,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proved page of a historical document's history in the
    /// layout the protocol version stores.
    pub fn verify_document_history(
        query: &DocumentHistoryQueryV1,
        proof: &DocumentHistoryProof,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, DocumentHistoryV1), Error> {
        match (
            platform_version
                .drive
                .methods
                .verify
                .document
                .verify_document_history,
            proof,
        ) {
            (0, DocumentHistoryProof::V0(proof)) => {
                let (start_at_ms, limit) = query.legacy_read()?;
                let (root, revisions) = Drive::verify_document_history_v0(
                    proof,
                    query.contract_id,
                    &query.document_type_name,
                    document_type,
                    query.document_id,
                    start_at_ms,
                    limit,
                    None,
                    platform_version,
                )?;
                let history = DocumentHistoryV1::from_legacy(revisions.unwrap_or_default())?;
                Ok((root, history))
            }
            (1, DocumentHistoryProof::V1(proof)) => {
                Drive::verify_document_history_v1(query, proof, document_type, platform_version)
            }
            (0 | 1, _) => Err(invalid(
                "the history proof layout does not match the protocol version",
            )),
            (version, _) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_document_history".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
