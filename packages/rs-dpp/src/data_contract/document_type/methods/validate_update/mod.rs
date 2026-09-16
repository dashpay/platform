use crate::data_contract::document_type::DocumentTypeRef;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod common;
mod v0;
mod v1;

impl DocumentTypeRef<'_> {
    /// Verify that the update to the document type is valid.
    /// We assume that new document type is valid.
    /// `new_contract_version` is the version the updated contract will have
    /// (already validated to be the old version + 1): a newly added required
    /// property must carry `requiredSince` equal to exactly that version.
    pub fn validate_update(
        &self,
        new_document_type: DocumentTypeRef,
        new_contract_version: u32,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .document_type
            .validate_update
        {
            0 => self.validate_update_v0(new_document_type, platform_version),
            1 => self.validate_update_v1(new_document_type, new_contract_version, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_update".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
