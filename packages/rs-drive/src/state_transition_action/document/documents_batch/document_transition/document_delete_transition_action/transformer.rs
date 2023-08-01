use std::sync::Arc;
use dpp::platform_value::Identifier;
use dpp::data_contract::DataContract;
use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentDeleteTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::{DocumentDeleteTransitionAction, DocumentDeleteTransitionActionV0};

impl DocumentDeleteTransitionAction {
    pub fn from_document_create_transition_with_contract_lookup(
        value: DocumentDeleteTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentDeleteTransition::V0(v0) => Ok(DocumentDeleteTransitionActionV0::try_from_document_delete_transition_with_contract_lookup(v0, get_data_contract)?.into()),
        }
    }

    pub fn from_document_borrowed_create_transition_with_contract_lookup(
        value: &DocumentDeleteTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentDeleteTransition::V0(v0) => Ok(DocumentDeleteTransitionActionV0::try_from_borrowed_document_delete_transition_with_contract_lookup(v0, get_data_contract)?.into()),
        }
    }
}
