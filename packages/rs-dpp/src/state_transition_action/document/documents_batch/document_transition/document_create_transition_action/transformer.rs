use platform_value::Identifier;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionV0};

impl<'a> DocumentCreateTransitionAction<'a> {
    pub fn from_document_create_transition_with_contract_lookup(
        value: DocumentCreateTransition,
        get_data_contract: impl FnMut(Identifier) -> Result<&'a DataContract, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentCreateTransition::V0(v0) => Ok(DocumentCreateTransitionActionV0::try_from_document_create_transition_with_contract_lookup(v0, get_data_contract)?.into()),
        }
    }

    pub fn from_document_borrowed_create_transition_with_contract_lookup(
        value: &DocumentCreateTransition,
        get_data_contract: impl FnMut(Identifier) -> Result<&'a DataContract, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentCreateTransition::V0(v0) => Ok(DocumentCreateTransitionActionV0::try_from_borrowed_document_create_transition_with_contract_lookup(v0, get_data_contract)?.into()),
        }
    }
}
