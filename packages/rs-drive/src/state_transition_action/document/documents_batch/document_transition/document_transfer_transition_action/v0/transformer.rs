use dpp::block::block_info::BlockInfo;
use dpp::document::property_names::PRICE;
use dpp::document::{property_names, Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_transition::document_transfer_transition::DocumentTransferTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_transfer_transition_action::v0::DocumentTransferTransitionActionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};

impl DocumentTransferTransitionActionV0 {
    /// try from borrowed
    pub fn try_from_borrowed_document_transfer_transition(
        document_transfer_transition: &DocumentTransferTransitionV0,
        original_document: Document,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentTransferTransitionV0 {
            base,
            recipient_owner_id,
            ..
        } = document_transfer_transition;
        let base =
            DocumentBaseTransitionAction::from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?;

        let mut modified_document = original_document;

        modified_document.set_owner_id(*recipient_owner_id);

        // We must remove the price
        modified_document.properties_mut().remove(PRICE);

        modified_document.bump_revision();

        if base.document_type_field_is_required(property_names::TRANSFERRED_AT)? {
            modified_document.set_transferred_at(Some(block_info.time_ms));
        }

        if base.document_type_field_is_required(property_names::TRANSFERRED_AT_BLOCK_HEIGHT)? {
            modified_document.set_transferred_at_block_height(Some(block_info.height));
        }

        if base.document_type_field_is_required(property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT)? {
            modified_document.set_transferred_at_core_block_height(Some(block_info.core_height));
        }

        Ok(DocumentTransferTransitionActionV0 {
            base,
            document: modified_document,
        })
    }
}
