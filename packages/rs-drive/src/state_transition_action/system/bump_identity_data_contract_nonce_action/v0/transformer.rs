use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Identifier;
use dpp::prelude::UserFeeMultiplier;

use dpp::ProtocolError;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionV0;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionV0;

impl BumpIdentityDataContractNonceActionV0 {
    /// try from base transition
    pub fn try_from_base_transition(
        value: DocumentBaseTransitionV0,
        identity_id: Identifier,
        fee_multiplier: UserFeeMultiplier,
    ) -> Result<Self, ProtocolError> {
        let DocumentBaseTransitionV0 {
            data_contract_id,
            identity_contract_nonce,
            ..
        } = value;
        Ok(BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id,
            identity_contract_nonce,
            fee_multiplier,
        })
    }

    /// try from borrowed base transition
    pub fn try_from_borrowed_base_transition(
        value: &DocumentBaseTransitionV0,
        identity_id: Identifier,
        fee_multiplier: UserFeeMultiplier,
    ) -> Result<Self, ProtocolError> {
        let DocumentBaseTransitionV0 {
            data_contract_id,
            identity_contract_nonce,
            ..
        } = value;
        Ok(BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: *data_contract_id,
            identity_contract_nonce: *identity_contract_nonce,
            fee_multiplier,
        })
    }

    /// try from base transition
    pub fn try_from_base_transition_action(
        value: DocumentBaseTransitionActionV0,
        identity_id: Identifier,
        fee_multiplier: UserFeeMultiplier,
    ) -> Result<Self, ProtocolError> {
        let DocumentBaseTransitionActionV0 {
            data_contract,
            identity_contract_nonce,
            ..
        } = value;
        Ok(BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: data_contract.contract.id(),
            identity_contract_nonce,
            fee_multiplier,
        })
    }

    /// try from borrowed base transition
    pub fn try_from_borrowed_base_transition_action(
        value: &DocumentBaseTransitionActionV0,
        identity_id: Identifier,
        fee_multiplier: UserFeeMultiplier,
    ) -> Result<Self, ProtocolError> {
        let DocumentBaseTransitionActionV0 {
            data_contract,
            identity_contract_nonce,
            ..
        } = value;
        Ok(BumpIdentityDataContractNonceActionV0 {
            identity_id,
            data_contract_id: data_contract.contract.id(),
            identity_contract_nonce: *identity_contract_nonce,
            fee_multiplier,
        })
    }

    /// try from data contract update
    pub fn try_from_data_contract_update(
        value: DataContractUpdateTransitionV0,
    ) -> Result<Self, ProtocolError> {
        let DataContractUpdateTransitionV0 {
            data_contract,
            identity_contract_nonce,
            fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce,
            fee_multiplier,
        })
    }

    /// try from borrowed data contract update
    pub fn try_from_borrowed_data_contract_update(
        value: &DataContractUpdateTransitionV0,
    ) -> Result<Self, ProtocolError> {
        let DataContractUpdateTransitionV0 {
            data_contract,
            identity_contract_nonce,
            fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce: *identity_contract_nonce,
            fee_multiplier: *fee_multiplier,
        })
    }

    /// try from data contract update action
    pub fn try_from_data_contract_update_action(
        value: DataContractUpdateTransitionActionV0,
    ) -> Result<Self, ProtocolError> {
        let DataContractUpdateTransitionActionV0 {
            data_contract,
            identity_contract_nonce,
            fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce,
            fee_multiplier,
        })
    }

    /// try from borrowed data contract update action
    pub fn try_from_borrowed_data_contract_update_action(
        value: &DataContractUpdateTransitionActionV0,
    ) -> Result<Self, ProtocolError> {
        let DataContractUpdateTransitionActionV0 {
            data_contract,
            identity_contract_nonce,
            fee_multiplier,
            ..
        } = value;
        Ok(BumpIdentityDataContractNonceActionV0 {
            identity_id: data_contract.owner_id(),
            data_contract_id: data_contract.id(),
            identity_contract_nonce: *identity_contract_nonce,
            fee_multiplier: *fee_multiplier,
        })
    }
}
