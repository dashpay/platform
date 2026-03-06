use crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldedTransferTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            ShieldedTransferTransition::V0(v0) => v0.validate_structure(platform_version),
        }
    }
}
