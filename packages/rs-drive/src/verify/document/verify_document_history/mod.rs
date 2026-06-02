mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies that the document's history is included in the proof.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_document_history(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        document_type: DocumentTypeRef,
        document_id: [u8; 32],
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<BTreeMap<u64, Document>>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document
            .verify_document_history
        {
            0 => Drive::verify_document_history_v0(
                proof,
                contract_id,
                document_type_name,
                document_type,
                document_id,
                start_at_ms,
                limit,
                offset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_document_history".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
