use crate::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for IdentityTopUpFromShieldedPoolTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => {
                v0.validate_structure(platform_version)
            }
        }
    }
}
