use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Identifier;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use crate::state_transition_action::document::documents_batch::document_transition::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionV0;

impl BumpIdentityDataContractNonceActionV0 {
    /// try from base transition
    pub fn try_from_base_transition(
        value: DocumentBaseTransitionV0,
        identity_id: Identifier,
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
        })
    }

    /// try from borrowed base transition
    pub fn try_from_borrowed_base_transition(
        value: &DocumentBaseTransitionV0,
        identity_id: Identifier,
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
        })
    }

    /// try from base transition
    pub fn try_from_base_transition_action(
        value: DocumentBaseTransitionActionV0,
        identity_id: Identifier,
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
        })
    }

    /// try from borrowed base transition
    pub fn try_from_borrowed_base_transition_action(
        value: &DocumentBaseTransitionActionV0,
        identity_id: Identifier,
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
        })
    }
}
