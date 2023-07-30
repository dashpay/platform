use platform_value::Identifier;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use crate::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionV0;

impl<'a> DocumentDeleteTransitionActionV0<'a> {
    pub(in crate::state_transition_action::document::documents_batch::document_transition) fn try_from_document_delete_transition_with_contract_lookup(
        value: DocumentDeleteTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<&'a DataContract, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentDeleteTransitionV0 { base, .. } = value;
        Ok(DocumentDeleteTransitionActionV0 {
            base: DocumentBaseTransitionAction::from_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?,
        })
    }

    pub(in crate::state_transition_action::document::documents_batch::document_transition) fn try_from_borrowed_document_delete_transition_with_contract_lookup(
        value: &DocumentDeleteTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<&'a DataContract, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentDeleteTransitionV0 { base, .. } = value;
        Ok(DocumentDeleteTransitionActionV0 {
            base: DocumentBaseTransitionAction::from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?,
        })
    }
}
