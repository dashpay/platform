use dpp::balances::credits::TokenAmount;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::TokenContractPosition;
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use dpp::tokens::calculate_token_id;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionV0;

impl DocumentBaseTransitionActionV0 {
    /// try from base transition with contract lookup
    pub fn try_from_base_transition_with_contract_lookup(
        value: DocumentBaseTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        get_token_cost: impl Fn(&DocumentType) -> Option<(TokenContractPosition, TokenAmount)>,
    ) -> Result<Self, ProtocolError> {
        let DocumentBaseTransitionV0 {
            id,
            document_type_name,
            data_contract_id,
            identity_contract_nonce,
        } = value;
        let data_contract = get_data_contract(data_contract_id)?;
        let document_type = data_contract
            .contract
            .document_type_borrowed_for_name(document_type_name.as_str())?;
        let token_cost =
            get_token_cost(document_type).map(|(token_contract_position, token_amount)| {
                (
                    calculate_token_id(data_contract_id.as_bytes(), token_contract_position).into(),
                    token_amount,
                )
            });
        Ok(DocumentBaseTransitionActionV0 {
            id,
            identity_contract_nonce,
            document_type_name,
            data_contract,
            token_cost,
        })
    }

    /// try from borrowed base transition with contract lookup
    pub fn try_from_borrowed_base_transition_with_contract_lookup(
        value: &DocumentBaseTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        get_token_cost: impl Fn(&DocumentType) -> Option<(TokenContractPosition, TokenAmount)>,
    ) -> Result<Self, ProtocolError> {
        let DocumentBaseTransitionV0 {
            id,
            document_type_name,
            data_contract_id,
            identity_contract_nonce,
        } = value;
        let data_contract = get_data_contract(*data_contract_id)?;
        let document_type = data_contract
            .contract
            .document_type_borrowed_for_name(document_type_name)?;
        let token_cost =
            get_token_cost(document_type).map(|(token_contract_position, token_amount)| {
                (
                    calculate_token_id(data_contract_id.as_bytes(), token_contract_position).into(),
                    token_amount,
                )
            });
        Ok(DocumentBaseTransitionActionV0 {
            id: *id,
            identity_contract_nonce: *identity_contract_nonce,
            document_type_name: document_type_name.clone(),
            data_contract,
            token_cost,
        })
    }
}
