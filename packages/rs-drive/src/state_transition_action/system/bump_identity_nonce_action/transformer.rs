use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use crate::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use crate::state_transition_action::system::bump_identity_nonce_action::{
    BumpIdentityNonceAction, BumpIdentityNonceActionV0,
};
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;

impl BumpIdentityNonceAction {
    /// from identity update
    pub fn from_identity_update_transition(value: IdentityUpdateTransition) -> Self {
        match value {
            IdentityUpdateTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_update(v0).into()
            }
        }
    }

    /// from borrowed identity update
    pub fn from_borrowed_identity_update_transition(value: &IdentityUpdateTransition) -> Self {
        match value {
            IdentityUpdateTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_update(v0).into()
            }
        }
    }

    /// from identity update action
    pub fn from_identity_update_transition_action(value: IdentityUpdateTransitionAction) -> Self {
        match value {
            IdentityUpdateTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_update_action(v0).into()
            }
        }
    }

    /// from borrowed identity update action
    pub fn from_borrowed_identity_update_transition_action(
        value: &IdentityUpdateTransitionAction,
    ) -> Self {
        match value {
            IdentityUpdateTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_update_action(v0).into()
            }
        }
    }

    /// from data contract create transition
    pub fn from_data_contract_create_transition(value: DataContractCreateTransition) -> Self {
        match value {
            DataContractCreateTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_contract_create(v0).into()
            }
        }
    }

    /// from borrowed data contract create transition
    pub fn from_borrowed_data_contract_create_transition(
        value: &DataContractCreateTransition,
    ) -> Self {
        match value {
            DataContractCreateTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_contract_create(v0).into()
            }
        }
    }

    /// from data contract create transition action
    pub fn from_data_contract_create_action(value: DataContractCreateTransitionAction) -> Self {
        match value {
            DataContractCreateTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_contract_create_action(v0).into()
            }
        }
    }

    /// from borrowed data contract create transition action
    pub fn from_borrowed_data_contract_create_action(
        value: &DataContractCreateTransitionAction,
    ) -> Self {
        match value {
            DataContractCreateTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_contract_create_action(v0).into()
            }
        }
    }

    /// from identity transfer
    pub fn from_identity_credit_transfer_transition(
        value: IdentityCreditTransferTransition,
    ) -> Self {
        match value {
            IdentityCreditTransferTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_credit_transfer(v0).into()
            }
        }
    }

    /// from borrowed identity transfer
    pub fn from_borrowed_identity_credit_transfer_transition(
        value: &IdentityCreditTransferTransition,
    ) -> Self {
        match value {
            IdentityCreditTransferTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_credit_transfer(v0).into()
            }
        }
    }

    /// from identity transfer action
    pub fn from_identity_credit_transfer_transition_action(
        value: IdentityCreditTransferTransitionAction,
    ) -> Self {
        match value {
            IdentityCreditTransferTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_credit_transfer_action(v0).into()
            }
        }
    }

    /// from borrowed identity transfer action
    pub fn from_borrowed_identity_credit_transfer_transition_action(
        value: &IdentityCreditTransferTransitionAction,
    ) -> Self {
        match value {
            IdentityCreditTransferTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_credit_transfer_action(v0).into()
            }
        }
    }

    /// from identity withdrawal
    pub fn from_identity_credit_withdrawal_transition(
        value: IdentityCreditWithdrawalTransition,
    ) -> Self {
        match value {
            IdentityCreditWithdrawalTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_credit_withdrawal(v0).into()
            }
        }
    }

    /// from borrowed identity withdrawal
    pub fn from_borrowed_identity_credit_withdrawal_transition(
        value: &IdentityCreditWithdrawalTransition,
    ) -> Self {
        match value {
            IdentityCreditWithdrawalTransition::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_credit_withdrawal(v0).into()
            }
        }
    }

    /// from identity withdrawal action
    pub fn from_identity_credit_withdrawal_transition_action(
        value: IdentityCreditWithdrawalTransitionAction,
    ) -> Self {
        match value {
            IdentityCreditWithdrawalTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_identity_credit_withdrawal_action(v0).into()
            }
        }
    }

    /// from borrowed identity withdrawal action
    pub fn from_borrowed_identity_credit_withdrawal_transition_action(
        value: &IdentityCreditWithdrawalTransitionAction,
    ) -> Self {
        match value {
            IdentityCreditWithdrawalTransitionAction::V0(v0) => {
                BumpIdentityNonceActionV0::from_borrowed_identity_credit_withdrawal_action(v0)
                    .into()
            }
        }
    }
}
