use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
use crate::state_transition_action::address_funds::address_funds_transfer::AddressFundsTransferTransitionAction;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;

impl From<AddressFundsTransferTransition> for AddressFundsTransferTransitionAction {
    fn from(value: AddressFundsTransferTransition) -> Self {
        match value {
            AddressFundsTransferTransition::V0(v0) => {
                AddressFundsTransferTransitionActionV0::from(v0).into()
            }
        }
    }
}

impl From<&AddressFundsTransferTransition> for AddressFundsTransferTransitionAction {
    fn from(value: &AddressFundsTransferTransition) -> Self {
        match value {
            AddressFundsTransferTransition::V0(v0) => {
                AddressFundsTransferTransitionActionV0::from(v0).into()
            }
        }
    }
}
