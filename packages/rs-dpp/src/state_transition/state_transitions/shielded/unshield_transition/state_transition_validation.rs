use crate::state_transition::unshield_transition::UnshieldTransition;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for UnshieldTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            UnshieldTransition::V0(v0) => v0.validate_structure(platform_version),
        }
    }
}
