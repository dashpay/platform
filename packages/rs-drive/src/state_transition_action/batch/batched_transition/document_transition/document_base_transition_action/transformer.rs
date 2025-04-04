use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::data_contract::document_type::DocumentType;
use dpp::prelude::ConsensusValidationResult;
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenCost;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};

impl DocumentBaseTransitionAction {
    /// from borrowed base transition with contract lookup
    pub fn try_from_borrowed_base_transition_with_contract_lookup(
        value: &DocumentBaseTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        get_token_cost: impl Fn(&DocumentType) -> Option<DocumentActionTokenCost>,
        action: &str,
    ) -> Result<ConsensusValidationResult<Self>, Error> {
        Ok(
            DocumentBaseTransitionActionV0::try_from_borrowed_base_transition_with_contract_lookup(
                value,
                get_data_contract,
                get_token_cost,
                action,
            )?
            .map(|v0| v0.into()),
        )
    }
}
