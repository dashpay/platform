use crate::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use crate::state_transition::{
    StateTransitionStructureValidation, StateTransitionWitnessValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for IdentityTopUpFromAddressesTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            IdentityTopUpFromAddressesTransition::V0(v0) => v0.validate_structure(platform_version),
        }
    }
}

impl StateTransitionWitnessValidation for IdentityTopUpFromAddressesTransition {}
