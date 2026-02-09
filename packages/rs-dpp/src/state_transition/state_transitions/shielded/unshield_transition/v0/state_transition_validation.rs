use crate::state_transition::unshield_transition::v0::UnshieldTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for UnshieldTransitionV0 {
    fn validate_structure(
        &self,
        _platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // TODO: Add proper structure validation for unshield transition
        SimpleConsensusValidationResult::new()
    }
}
