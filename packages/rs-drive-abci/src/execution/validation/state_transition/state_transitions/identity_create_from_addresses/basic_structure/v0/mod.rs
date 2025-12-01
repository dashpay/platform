use crate::error::Error;
use dpp::consensus::basic::state_transition::{
    InputWitnessCountMismatchError, TransitionOverMaxInputsError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use dpp::consensus::state::state_error::StateError;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::{
    StateTransitionAddressInputs, StateTransitionStructureValidation, StateTransitionWitnessSigned,
};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create_from_addresses) trait IdentityCreateFromAddressesStateTransitionBasicStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreateFromAddressesStateTransitionBasicStructureValidationV0
    for IdentityCreateFromAddressesTransition
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        Ok(self.validate_structure(platform_version))
    }
}
