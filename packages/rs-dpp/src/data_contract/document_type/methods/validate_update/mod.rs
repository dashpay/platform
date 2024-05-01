use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

impl<'a> DocumentTypeRef<'a> {
    pub fn validate_update(
        &self,
        new_document_type: &DocumentType,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .document_type
            .validate_update
        {
            0 => Ok(self.validate_update_v0(new_document_type)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_config_update".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
