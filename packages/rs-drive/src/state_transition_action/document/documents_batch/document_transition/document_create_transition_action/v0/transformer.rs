use std::sync::Arc;
use dpp::platform_value::Identifier;
use dpp::data_contract::DataContract;
use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionActionV0;

impl DocumentCreateTransitionActionV0 {
    pub fn try_from_document_create_transition_with_contract_lookup(
        value: DocumentCreateTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionV0 {
            base,
            created_at,
            updated_at,
            data,
            ..
        } = value;
        Ok(DocumentCreateTransitionActionV0 {
            base: DocumentBaseTransitionAction::from_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?,
            created_at,
            updated_at,
            data,
        })
    }

    pub fn try_from_borrowed_document_create_transition_with_contract_lookup(
        value: &DocumentCreateTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionV0 {
            base,
            created_at,
            updated_at,
            data,
            ..
        } = value;
        Ok(DocumentCreateTransitionActionV0 {
            base: DocumentBaseTransitionAction::from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?,
            created_at: *created_at,
            updated_at: *updated_at,
            //todo: get rid of clone
            data: data.clone(),
        })
    }
}
