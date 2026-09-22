use crate::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldFromIdentityTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.validate_structure(platform_version),
        }
    }
}
