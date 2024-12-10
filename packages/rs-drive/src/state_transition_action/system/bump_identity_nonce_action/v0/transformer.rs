use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;
use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use crate::state_transition_action::identity::identity_update::v0::IdentityUpdateTransitionActionV0;
use crate::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceActionV0;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use dpp::state_transition::StateTransitionLike;

impl BumpIdentityNonceActionV0 {
    /// from identity update
    pub fn from_identity_update(value: IdentityUpdateTransitionV0) -> Self {
        let IdentityUpdateTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            user_fee_increase,
        }
    }

    /// from borrowed identity update
    pub fn from_borrowed_identity_update(value: &IdentityUpdateTransitionV0) -> Self {
        let IdentityUpdateTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from identity update action
    pub fn from_identity_update_action(value: IdentityUpdateTransitionActionV0) -> Self {
        let IdentityUpdateTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            user_fee_increase,
        }
    }

    /// from borrowed identity update action
    pub fn from_borrowed_identity_update_action(value: &IdentityUpdateTransitionActionV0) -> Self {
        let IdentityUpdateTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from contract create
    pub fn from_contract_create(value: DataContractCreateTransitionV0) -> Self {
        let DataContractCreateTransitionV0 {
            data_contract,
            identity_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: data_contract.owner_id(),
            identity_nonce,
            user_fee_increase,
        }
    }

    /// from borrowed contract create
    pub fn from_borrowed_contract_create(value: &DataContractCreateTransitionV0) -> Self {
        let DataContractCreateTransitionV0 {
            data_contract,
            identity_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: data_contract.owner_id(),
            identity_nonce: *identity_nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from contract create action
    pub fn from_contract_create_action(value: DataContractCreateTransitionActionV0) -> Self {
        let DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: data_contract.owner_id(),
            identity_nonce,
            user_fee_increase,
        }
    }

    /// from contract create action
    pub fn from_borrowed_contract_create_action(
        value: &DataContractCreateTransitionActionV0,
    ) -> Self {
        let DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: data_contract.owner_id(),
            identity_nonce: *identity_nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from identity credit transfer
    pub fn from_identity_credit_transfer(value: IdentityCreditTransferTransitionV0) -> Self {
        let IdentityCreditTransferTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            user_fee_increase,
        }
    }

    /// from borrowed identity credit transfer
    pub fn from_borrowed_identity_credit_transfer(
        value: &IdentityCreditTransferTransitionV0,
    ) -> Self {
        let IdentityCreditTransferTransitionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from identity credit transfer action
    pub fn from_identity_credit_transfer_action(
        value: IdentityCreditTransferTransitionActionV0,
    ) -> Self {
        let IdentityCreditTransferTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            user_fee_increase,
        }
    }

    /// from borrowed identity credit transfer action
    pub fn from_borrowed_identity_credit_transfer_action(
        value: &IdentityCreditTransferTransitionActionV0,
    ) -> Self {
        let IdentityCreditTransferTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from identity credit withdrawal
    pub fn from_identity_credit_withdrawal(value: IdentityCreditWithdrawalTransition) -> Self {
        BumpIdentityNonceActionV0 {
            identity_id: value.identity_id(),
            identity_nonce: value.nonce(),
            user_fee_increase: value.user_fee_increase(),
        }
    }

    /// from borrowed identity credit withdrawal
    pub fn from_borrowed_identity_credit_withdrawal(
        value: &IdentityCreditWithdrawalTransition,
    ) -> Self {
        BumpIdentityNonceActionV0 {
            identity_id: value.identity_id(),
            identity_nonce: value.nonce(),
            user_fee_increase: value.user_fee_increase(),
        }
    }

    /// from identity credit withdrawal action
    pub fn from_identity_credit_withdrawal_action(
        value: IdentityCreditWithdrawalTransitionActionV0,
    ) -> Self {
        let IdentityCreditWithdrawalTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id,
            identity_nonce: nonce,
            user_fee_increase,
        }
    }

    /// from borrowed identity credit withdrawal action
    pub fn from_borrowed_identity_credit_withdrawal_action(
        value: &IdentityCreditWithdrawalTransitionActionV0,
    ) -> Self {
        let IdentityCreditWithdrawalTransitionActionV0 {
            identity_id,
            nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityNonceActionV0 {
            identity_id: *identity_id,
            identity_nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }
}
