use platform_version::version::PlatformVersion;
use crate::ProtocolError;
use crate::state_transition::batch_transition::token_config_update_transition::validate_structure::v0::TokenConfigUpdateTransitionStructureValidationV0;
use crate::state_transition::batch_transition::TokenConfigUpdateTransition;
use crate::validation::SimpleConsensusValidationResult;
mod v0;

pub trait TokenConfigUpdateTransitionStructureValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenConfigUpdateTransitionStructureValidation for TokenConfigUpdateTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_config_update_transition_structure_validation
        {
            0 => self.validate_structure_v0(platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenConfigUpdateTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
