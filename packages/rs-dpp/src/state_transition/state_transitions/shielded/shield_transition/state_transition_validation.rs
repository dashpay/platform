use crate::state_transition::shield_transition::ShieldTransition;
use crate::state_transition::{
    StateTransitionStructureValidation, StateTransitionWitnessValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            ShieldTransition::V0(v0) => v0.validate_structure(platform_version),
        }
    }
}

impl StateTransitionWitnessValidation for ShieldTransition {}
