use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};

impl DocumentBaseTransitionAction {
    /// from base transition with contract lookup
    pub fn try_from_base_transition_with_contract_lookup(
        value: DocumentBaseTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentBaseTransition::V0(v0) => Ok(
                DocumentBaseTransitionActionV0::try_from_base_transition_with_contract_lookup(
                    v0,
                    get_data_contract,
                )?
                .into(),
            ),
        }
    }

    /// from borrowed base transition with contract lookup
    pub fn try_from_borrowed_base_transition_with_contract_lookup(
        value: &DocumentBaseTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentBaseTransition::V0(v0) => Ok(DocumentBaseTransitionActionV0::try_from_borrowed_base_transition_with_contract_lookup(v0, get_data_contract)?.into()),
        }
    }
}
