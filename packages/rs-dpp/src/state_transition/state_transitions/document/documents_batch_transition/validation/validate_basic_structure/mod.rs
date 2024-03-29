use crate::state_transition::documents_batch_transition::DocumentsBatchTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

impl DocumentsBatchTransition {
    pub fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .documents
            .documents_batch_transition
            .validation
            .validate_basic_structure
        {
            0 => self.validate_basic_structure_v0(platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::validate".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
