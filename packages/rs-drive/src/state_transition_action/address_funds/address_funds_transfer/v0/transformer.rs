use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;

impl From<AddressFundsTransferTransitionV0> for AddressFundsTransferTransitionActionV0 {
    fn from(value: AddressFundsTransferTransitionV0) -> Self {
        let AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            user_fee_increase,
            ..
        } = value;
        AddressFundsTransferTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            outputs,
            user_fee_increase,
        }
    }
}

impl From<&AddressFundsTransferTransitionV0> for AddressFundsTransferTransitionActionV0 {
    fn from(value: &AddressFundsTransferTransitionV0) -> Self {
        let AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            user_fee_increase,
            ..
        } = value;
        AddressFundsTransferTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            outputs: outputs.clone(),
            user_fee_increase: *user_fee_increase,
        }
    }
}
