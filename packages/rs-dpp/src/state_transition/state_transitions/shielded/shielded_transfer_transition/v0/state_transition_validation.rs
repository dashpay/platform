use crate::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldedTransferTransitionV0 {
    fn validate_structure(
        &self,
        _platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // TODO: Add proper structure validation for shielded transfer transition
        SimpleConsensusValidationResult::new()
    }
}
