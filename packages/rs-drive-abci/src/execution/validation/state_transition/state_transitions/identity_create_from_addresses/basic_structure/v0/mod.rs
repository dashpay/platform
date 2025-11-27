use crate::error::Error;
use dpp::consensus::basic::state_transition::{
    InputWitnessCountMismatchError, TransitionOverMaxInputsError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use dpp::consensus::state::state_error::StateError;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::{StateTransitionAddressInputs, StateTransitionWitnessSigned};
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
        if self.inputs().len() > platform_version.dpp.state_transitions.max_inputs as usize {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionOverMaxInputsError(TransitionOverMaxInputsError::new(
                    self.inputs().len().min(u16::MAX as usize) as u16,
                    platform_version.dpp.state_transitions.max_inputs,
                ))
                .into(),
            ));
        }

        if self.inputs().len() != self.witnesses().len() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                BasicError::InputWitnessCountMismatchError(InputWitnessCountMismatchError::new(
                    self.inputs().len().min(u16::MAX as usize) as u16,
                    self.witnesses().len().min(u16::MAX as usize) as u16,
                ))
                .into(),
            ));
        }

        if self.public_keys().len()
            > platform_version
                .dpp
                .state_transitions
                .identities
                .max_public_keys_in_creation as usize
        {
            Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::MaxIdentityPublicKeyLimitReachedError(
                    MaxIdentityPublicKeyLimitReachedError::new(
                        platform_version
                            .dpp
                            .state_transitions
                            .identities
                            .max_public_keys_in_creation as usize,
                    ),
                )
                .into(),
            ))
        } else {
            Ok(SimpleConsensusValidationResult::new())
        }
    }
}
