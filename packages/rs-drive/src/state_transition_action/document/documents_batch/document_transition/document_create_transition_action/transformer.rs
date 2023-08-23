use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionV0};

impl DocumentCreateTransitionAction {
    pub fn from_document_create_transition_with_contract_lookup(
        value: DocumentCreateTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentCreateTransition::V0(v0) => Ok(DocumentCreateTransitionActionV0::try_from_document_create_transition_with_contract_lookup(v0, get_data_contract)?.into()),
        }
    }

    pub fn from_document_borrowed_create_transition_with_contract_lookup(
        value: &DocumentCreateTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentCreateTransition::V0(v0) => Ok(DocumentCreateTransitionActionV0::try_from_borrowed_document_create_transition_with_contract_lookup(v0, get_data_contract)?.into()),
        }
    }
}
