use crate::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for IdentityCreateFromShieldedPoolTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => {
                v0.validate_structure(platform_version)
            }
        }
    }
}
