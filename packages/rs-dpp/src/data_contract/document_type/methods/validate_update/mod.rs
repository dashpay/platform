use crate::data_contract::document_type::DocumentTypeRef;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

impl<'a> DocumentTypeRef<'a> {
    /// Verify that the update to the document type is valid.
    /// We assume that new document type is valid
    pub fn validate_update(
        &self,
        new_document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .document_type
            .validate_update
        {
            0 => self.validate_update_v0(new_document_type, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_update".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
