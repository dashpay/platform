use crate::state_transition::batch_transition::token_burn_transition::validate_structure::v0::TokenBurnTransitionActionStructureValidationV0;
use crate::state_transition::batch_transition::TokenBurnTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
mod v0;

pub trait TokenBurnTransitionStructureValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenBurnTransitionStructureValidation for TokenBurnTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_burn_transition_structure_validation
        {
            0 => self.validate_structure_v0(),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenBurnTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
