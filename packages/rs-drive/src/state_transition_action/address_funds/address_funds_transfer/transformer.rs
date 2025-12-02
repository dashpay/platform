use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
use crate::state_transition_action::address_funds::address_funds_transfer::AddressFundsTransferTransitionAction;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use std::collections::BTreeMap;

impl AddressFundsTransferTransitionAction {
    /// Transforms the state transition into an action using pre-validated inputs with remaining balances.
    pub fn try_from_transition(
        value: &AddressFundsTransferTransition,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> ConsensusValidationResult<Self> {
        match value {
            AddressFundsTransferTransition::V0(v0) => {
                let result = AddressFundsTransferTransitionActionV0::try_from_transition(
                    v0,
                    inputs_with_remaining_balance,
                );
                result.map(|action| action.into())
            }
        }
    }
}
