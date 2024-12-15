use std::sync::Arc;

use dpp::platform_value::Identifier;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionActionV0;

impl TokenBaseTransitionActionV0 {
    /// try from base transition with contract lookup
    pub fn try_from_base_transition_with_contract_lookup(
        value: TokenBaseTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let TokenBaseTransitionV0 {
            token_contract_position: token_id,
            data_contract_id,
            identity_contract_nonce,
        } = value;
        Ok(TokenBaseTransitionActionV0 {
            identity_contract_nonce,
            token_position: token_id,
            data_contract: get_data_contract(data_contract_id)?,
        })
    }

    /// try from borrowed base transition with contract lookup
    pub fn try_from_borrowed_base_transition_with_contract_lookup(
        value: &TokenBaseTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let TokenBaseTransitionV0 {
            token_contract_position: token_id,
            data_contract_id,
            identity_contract_nonce,
        } = value;
        Ok(TokenBaseTransitionActionV0 {
            identity_contract_nonce: *identity_contract_nonce,
            token_position: *token_id,
            data_contract: get_data_contract(*data_contract_id)?,
        })
    }
}
