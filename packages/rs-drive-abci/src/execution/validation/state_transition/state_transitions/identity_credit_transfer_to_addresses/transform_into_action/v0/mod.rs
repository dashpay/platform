use crate::error::Error;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use drive::state_transition_action::identity::identity_credit_transfer_to_addresses::IdentityCreditTransferToAddressesTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_transfer_to_addresses) trait IdentityCreditTransferToAddressesStateTransitionStateValidationV0 {
    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreditTransferToAddressesStateTransitionStateValidationV0
    for IdentityCreditTransferToAddressesTransition
{
    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        Ok(ConsensusValidationResult::new_with_data(
            IdentityCreditTransferToAddressesTransitionAction::from(self).into(),
        ))
    }
}
