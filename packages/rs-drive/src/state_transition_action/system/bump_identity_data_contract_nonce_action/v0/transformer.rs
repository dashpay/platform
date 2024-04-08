use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Identifier;
use dpp::prelude::UserFeeIncrease;

use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionV0;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionV0;

impl BumpIdentityDataContractNonceActionV0 {
    /// from base transition
    pub fn from_base_transition(
        value: DocumentBaseTransitionV0,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        let DocumentBaseTransitionV0 {
            data_contract_id,
            identity_contract_nonce,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id,
            identity_contract_nonce,
            user_fee_increase,
        }
    }

    /// from borrowed base transition
    pub fn from_borrowed_base_transition(
        value: &DocumentBaseTransitionV0,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        let DocumentBaseTransitionV0 {
            data_contract_id,
            identity_contract_nonce,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: *data_contract_id,
            identity_contract_nonce: *identity_contract_nonce,
            user_fee_increase,
        }
    }

    /// from base transition
    pub fn from_base_transition_action(
        value: DocumentBaseTransitionActionV0,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        let DocumentBaseTransitionActionV0 {
            data_contract,
            identity_contract_nonce,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: data_contract.contract.id(),
            identity_contract_nonce,
            user_fee_increase,
        }
    }

    /// from borrowed base transition
    pub fn from_borrowed_base_transition_action(
        value: &DocumentBaseTransitionActionV0,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        let DocumentBaseTransitionActionV0 {
            data_contract,
            identity_contract_nonce,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: data_contract.contract.id(),
            identity_contract_nonce: *identity_contract_nonce,
            user_fee_increase,
        }
    }

    /// from data contract update
    pub fn from_data_contract_update(value: DataContractUpdateTransitionV0) -> Self {
        let DataContractUpdateTransitionV0 {
            data_contract,
            identity_contract_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce,
            user_fee_increase,
        }
    }

    /// from borrowed data contract update
    pub fn from_borrowed_data_contract_update(value: &DataContractUpdateTransitionV0) -> Self {
        let DataContractUpdateTransitionV0 {
            data_contract,
            identity_contract_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce: *identity_contract_nonce,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// from data contract update action
    pub fn from_data_contract_update_action(value: DataContractUpdateTransitionActionV0) -> Self {
        let DataContractUpdateTransitionActionV0 {
            data_contract,
            identity_contract_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce,
            user_fee_increase,
        }
    }

    /// from borrowed data contract update action
    pub fn from_borrowed_data_contract_update_action(
        value: &DataContractUpdateTransitionActionV0,
    ) -> Self {
        let DataContractUpdateTransitionActionV0 {
            data_contract,
            identity_contract_nonce,
            user_fee_increase,
            ..
        } = value;
        BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce: *identity_contract_nonce,
            user_fee_increase: *user_fee_increase,
        }
    }
}
