use crate::state_transition_action::address_input::address_input_credit_transfer::AddressInputCreditTransferTransitionAction;
use crate::state_transition_action::address_input::address_input_credit_withdrawal::AddressInputCreditWithdrawalTransitionAction;
use crate::state_transition_action::address_input::address_input_update::AddressInputUpdateTransitionAction;
use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use crate::state_transition_action::system::bump_address_input_nonce_action::{
    BumpAddressInputNonceAction, BumpAddressInputNonceActionV0,
};
use dpp::state_transition::address_input_credit_transfer_transition::AddressInputCreditTransferTransition;
use dpp::state_transition::address_input_credit_withdrawal_transition::AddressInputCreditWithdrawalTransition;
use dpp::state_transition::address_input_update_transition::AddressInputUpdateTransition;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;

impl BumpAddressInputNonceAction {
    /// from address_input update
    pub fn from_address_input_update_transition(value: AddressInputUpdateTransition) -> Self {
        match value {
            AddressInputUpdateTransition::V0(v0) => {
                BumpAddressInputNonceActionV0::from_address_input_update(v0).into()
            }
        }
    }

    /// from borrowed address_input update
    pub fn from_borrowed_address_input_update_transition(
        value: &AddressInputUpdateTransition,
    ) -> Self {
        match value {
            AddressInputUpdateTransition::V0(v0) => {
                BumpAddressInputNonceActionV0::from_borrowed_address_input_update(v0).into()
            }
        }
    }

    /// from address_input update action
    pub fn from_address_input_update_transition_action(
        value: AddressInputUpdateTransitionAction,
    ) -> Self {
        match value {
            AddressInputUpdateTransitionAction::V0(v0) => {
                BumpAddressInputNonceActionV0::from_address_input_update_action(v0).into()
            }
        }
    }

    /// from borrowed address_input update action
    pub fn from_borrowed_address_input_update_transition_action(
        value: &AddressInputUpdateTransitionAction,
    ) -> Self {
        match value {
            AddressInputUpdateTransitionAction::V0(v0) => {
                BumpAddressInputNonceActionV0::from_borrowed_address_input_update_action(v0).into()
            }
        }
    }

    /// from data contract create transition
    pub fn from_data_contract_create_transition(value: DataContractCreateTransition) -> Self {
        match value {
            DataContractCreateTransition::V0(v0) => {
                BumpAddressInputNonceActionV0::from_contract_create(v0).into()
            }
        }
    }

    /// from borrowed data contract create transition
    pub fn from_borrowed_data_contract_create_transition(
        value: &DataContractCreateTransition,
    ) -> Self {
        match value {
            DataContractCreateTransition::V0(v0) => {
                BumpAddressInputNonceActionV0::from_borrowed_contract_create(v0).into()
            }
        }
    }

    /// from data contract create transition action
    pub fn from_data_contract_create_action(value: DataContractCreateTransitionAction) -> Self {
        match value {
            DataContractCreateTransitionAction::V0(v0) => {
                BumpAddressInputNonceActionV0::from_contract_create_action(v0).into()
            }
        }
    }

    /// from borrowed data contract create transition action
    pub fn from_borrowed_data_contract_create_action(
        value: &DataContractCreateTransitionAction,
    ) -> Self {
        match value {
            DataContractCreateTransitionAction::V0(v0) => {
                BumpAddressInputNonceActionV0::from_borrowed_contract_create_action(v0).into()
            }
        }
    }

    /// from address_input transfer
    pub fn from_address_input_credit_transfer_transition(
        value: AddressInputCreditTransferTransition,
    ) -> Self {
        match value {
            AddressInputCreditTransferTransition::V0(v0) => {
                BumpAddressInputNonceActionV0::from_address_input_credit_transfer(v0).into()
            }
        }
    }

    /// from borrowed address_input transfer
    pub fn from_borrowed_address_input_credit_transfer_transition(
        value: &AddressInputCreditTransferTransition,
    ) -> Self {
        match value {
            AddressInputCreditTransferTransition::V0(v0) => {
                BumpAddressInputNonceActionV0::from_borrowed_address_input_credit_transfer(v0)
                    .into()
            }
        }
    }

    /// from address_input transfer action
    pub fn from_address_input_credit_transfer_transition_action(
        value: AddressInputCreditTransferTransitionAction,
    ) -> Self {
        match value {
            AddressInputCreditTransferTransitionAction::V0(v0) => {
                BumpAddressInputNonceActionV0::from_address_input_credit_transfer_action(v0).into()
            }
        }
    }

    /// from borrowed address_input transfer action
    pub fn from_borrowed_address_input_credit_transfer_transition_action(
        value: &AddressInputCreditTransferTransitionAction,
    ) -> Self {
        match value {
            AddressInputCreditTransferTransitionAction::V0(v0) => {
                BumpAddressInputNonceActionV0::from_borrowed_address_input_credit_transfer_action(
                    v0,
                )
                .into()
            }
        }
    }

    /// from address_input withdrawal
    pub fn from_address_input_credit_withdrawal_transition(
        value: AddressInputCreditWithdrawalTransition,
    ) -> Self {
        BumpAddressInputNonceActionV0::from_address_input_credit_withdrawal(value).into()
    }

    /// from borrowed address_input withdrawal
    pub fn from_borrowed_address_input_credit_withdrawal_transition(
        value: &AddressInputCreditWithdrawalTransition,
    ) -> Self {
        BumpAddressInputNonceActionV0::from_borrowed_address_input_credit_withdrawal(value).into()
    }

    /// from address_input withdrawal action
    pub fn from_address_input_credit_withdrawal_transition_action(
        value: AddressInputCreditWithdrawalTransitionAction,
    ) -> Self {
        match value {
            AddressInputCreditWithdrawalTransitionAction::V0(v0) => {
                BumpAddressInputNonceActionV0::from_address_input_credit_withdrawal_action(v0)
                    .into()
            }
        }
    }

    /// from borrowed address_input withdrawal action
    pub fn from_borrowed_address_input_credit_withdrawal_transition_action(
        value: &AddressInputCreditWithdrawalTransitionAction,
    ) -> Self {
        match value {
            AddressInputCreditWithdrawalTransitionAction::V0(v0) => {
                BumpAddressInputNonceActionV0::from_borrowed_address_input_credit_withdrawal_action(
                    v0,
                )
                .into()
            }
        }
    }
}
