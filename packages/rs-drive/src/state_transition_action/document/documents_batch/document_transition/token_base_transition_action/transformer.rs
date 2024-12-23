use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionV0};

impl TokenBaseTransitionAction {
    /// from base transition with contract lookup
    pub fn try_from_base_transition_with_contract_lookup(
        value: TokenBaseTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            TokenBaseTransition::V0(v0) => Ok(
                TokenBaseTransitionActionV0::try_from_base_transition_with_contract_lookup(
                    v0,
                    get_data_contract,
                )?
                .into(),
            ),
        }
    }

    /// from borrowed base transition with contract lookup
    pub fn try_from_borrowed_base_transition_with_contract_lookup(
        value: &TokenBaseTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            TokenBaseTransition::V0(v0) => Ok(TokenBaseTransitionActionV0::try_from_borrowed_base_transition_with_contract_lookup(v0, get_data_contract)?.into()),
        }
    }
}
