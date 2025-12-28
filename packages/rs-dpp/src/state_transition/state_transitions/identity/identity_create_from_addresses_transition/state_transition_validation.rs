use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use crate::state_transition::{
    StateTransitionStructureValidation, StateTransitionWitnessValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for IdentityCreateFromAddressesTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                v0.validate_structure(platform_version)
            }
        }
    }
}

impl StateTransitionWitnessValidation for IdentityCreateFromAddressesTransition {}
