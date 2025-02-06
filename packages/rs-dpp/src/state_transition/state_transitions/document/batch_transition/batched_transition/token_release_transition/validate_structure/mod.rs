use crate::state_transition::batch_transition::token_release_transition::validate_structure::v0::TokenReleaseTransitionActionStructureValidationV0;
use crate::state_transition::batch_transition::TokenReleaseTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

pub trait TokenReleaseTransitionStructureValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenReleaseTransitionStructureValidation for TokenReleaseTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_release_transition_structure_validation
        {
            0 => self.validate_structure_v0(),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenReleaseTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
