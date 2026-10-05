use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_create_from_addresses::bump_input_nonces_with_penalty;
use crate::execution::validation::state_transition::identity_create_from_addresses::public_key_signatures::v0::IdentityCreateFromAddressesStateTransitionSignaturesValidationV0;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create_from_addresses) trait IdentityCreateFromAddressesStateTransitionAdvancedStructureValidationV1
{
    fn validate_advanced_structure_v1(
        &self,
        action: &IdentityCreateFromAddressesTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateFromAddressesStateTransitionAdvancedStructureValidationV1
    for IdentityCreateFromAddressesTransition
{
    /// Like v0, except that a key whose proof of possession fails is refused unpaid.
    ///
    /// The address witnesses cannot sign the proofs of possession, which sign the same bytes, so
    /// the owners of the inputs never signed the key signatures this check judges. A failure is
    /// therefore not charged to the inputs: the transition is refused like one whose witnesses
    /// fail. check_tx runs this check too, so such a transition does not reach a block.
    fn validate_advanced_structure_v1(
        &self,
        action: &IdentityCreateFromAddressesTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let validation_result =
            IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                self.public_keys(),
                true,
                platform_version,
            )
            .map_err(Error::Protocol)?;

        if !validation_result.is_valid() {
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .validation_of_added_keys_structure_failure;

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_input_nonces_with_penalty(self, action, penalty)?,
                validation_result.errors,
            ));
        }

        let validation_result = self
            .validate_identity_create_from_addresses_state_transition_signatures_v0(
                signable_bytes,
                execution_context,
            );

        if validation_result.is_valid() {
            Ok(ConsensusValidationResult::new())
        } else {
            // No action: the failure is refused unpaid
            Ok(ConsensusValidationResult::new_with_errors(
                validation_result.errors,
            ))
        }
    }
}
