use std::sync::Arc;
use dpp::platform_value::Identifier;
use dpp::data_contract::DataContract;
use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionV0;

impl DocumentDeleteTransitionActionV0 {
    pub fn try_from_document_delete_transition_with_contract_lookup(
        value: DocumentDeleteTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentDeleteTransitionV0 { base, .. } = value;
        Ok(DocumentDeleteTransitionActionV0 {
            base: DocumentBaseTransitionAction::from_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?,
        })
    }

    pub fn try_from_borrowed_document_delete_transition_with_contract_lookup(
        value: &DocumentDeleteTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
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
