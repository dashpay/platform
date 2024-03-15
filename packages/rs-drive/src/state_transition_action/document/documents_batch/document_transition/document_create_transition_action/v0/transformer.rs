use dpp::block::block_info::BlockInfo;
use dpp::document::property_names;
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_create_transition::v0::DocumentCreateTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionActionV0;

impl DocumentCreateTransitionActionV0 {
    /// try from document create transition with contract lookup
    pub fn try_from_document_create_transition_with_contract_lookup(
        value: DocumentCreateTransitionV0,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionV0 { base, data, .. } = value;
        let base = DocumentBaseTransitionAction::from_base_transition_with_contract_lookup(
            base,
            get_data_contract,
        )?;
        let created_at = if base.document_type_field_is_required(property_names::CREATED_AT)? {
            Some(block_info.time_ms)
        } else {
            None
        };

        Ok(DocumentCreateTransitionActionV0 {
            base,
            created_at,
            data,
        })
    }

    /// try from borrowed document create transition with contract lookup
    pub fn try_from_borrowed_document_create_transition_with_contract_lookup(
        value: &DocumentCreateTransitionV0,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionV0 { base, data, .. } = value;
        let base =
            DocumentBaseTransitionAction::from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?;
        let created_at = if base.document_type_field_is_required(property_names::CREATED_AT)? {
            Some(block_info.time_ms)
        } else {
            None
        };

        Ok(DocumentCreateTransitionActionV0 {
            base,
            created_at,
            //todo: get rid of clone
            data: data.clone(),
        })
    }
}
