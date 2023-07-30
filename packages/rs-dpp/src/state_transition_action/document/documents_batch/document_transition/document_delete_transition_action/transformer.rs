use platform_value::Identifier;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use crate::state_transition::documents_batch_transition::document_transition::DocumentDeleteTransition;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::{DocumentDeleteTransitionAction, DocumentDeleteTransitionActionV0};

impl<'a> DocumentDeleteTransitionAction<'a> {
    pub fn from_document_create_transition_with_contract_lookup(
        value: DocumentDeleteTransition,
        get_data_contract: impl FnMut(Identifier) -> Result<&'a DataContract, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentDeleteTransition::V0(v0) => Ok(DocumentDeleteTransitionActionV0::try_from_document_delete_transition_with_contract_lookup(v0, get_data_contract)?.into()),
        }
    }

    pub fn from_document_borrowed_create_transition_with_contract_lookup(
        value: &DocumentDeleteTransition,
        get_data_contract: impl FnMut(Identifier) -> Result<&'a DataContract, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentDeleteTransition::V0(v0) => Ok(DocumentDeleteTransitionActionV0::try_from_borrowed_document_delete_transition_with_contract_lookup(v0, get_data_contract)?.into()),
        }
    }
}
