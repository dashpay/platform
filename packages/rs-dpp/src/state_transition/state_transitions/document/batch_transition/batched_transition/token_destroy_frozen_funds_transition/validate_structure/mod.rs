use platform_version::version::PlatformVersion;
use crate::ProtocolError;
use crate::state_transition::batch_transition::token_destroy_frozen_funds_transition::validate_structure::v0::TokenDestroyFrozenFundsTransitionStructureValidationV0;
use crate::state_transition::batch_transition::TokenDestroyFrozenFundsTransition;
use crate::validation::SimpleConsensusValidationResult;

mod v0;

pub trait TokenDestroyFrozenFundsTransitionStructureValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenDestroyFrozenFundsTransitionStructureValidation for TokenDestroyFrozenFundsTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_destroy_frozen_funds_transition_structure_validation
        {
            0 => self.validate_structure_v0(),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenDestroyFrozenFundsTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
