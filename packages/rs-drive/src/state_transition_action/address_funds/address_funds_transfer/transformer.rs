use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
use crate::state_transition_action::address_funds::address_funds_transfer::AddressFundsTransferTransitionAction;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use std::collections::BTreeMap;

impl AddressFundsTransferTransitionAction {
    /// Transforms the state transition into an action by validating inputs against provided balances.
    pub fn try_from_transition(
        value: &AddressFundsTransferTransition,
        input_balances: BTreeMap<PlatformAddress, Credits>,
    ) -> ConsensusValidationResult<Self> {
        match value {
            AddressFundsTransferTransition::V0(v0) => {
                let result =
                    AddressFundsTransferTransitionActionV0::try_from_transition(v0, input_balances);
                result.map(|action| action.into())
            }
        }
    }
}
