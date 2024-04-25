use dpp::block::block_info::BlockInfo;
use dpp::document::property_names::PRICE;
use dpp::document::{property_names, Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_transition::document_purchase_transition::DocumentPurchaseTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_purchase_transition_action::v0::DocumentPurchaseTransitionActionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};

impl DocumentPurchaseTransitionActionV0 {
    /// try from borrowed
    pub fn try_from_borrowed_document_purchase_transition(
        document_purchase_transition: &DocumentPurchaseTransitionV0,
        original_document: Document,
        purchaser_id: Identifier,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentPurchaseTransitionV0 { base, price, .. } = document_purchase_transition;
        let base =
            DocumentBaseTransitionAction::from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?;

        let original_owner_id = original_document.owner_id();

        let mut modified_document = original_document;

        modified_document.bump_revision();

        // We must remove the price if there is one
        modified_document.properties_mut().remove(PRICE);

        modified_document.set_owner_id(purchaser_id);

        if base.document_type_field_is_required(property_names::TRANSFERRED_AT)? {
            modified_document.set_transferred_at(Some(block_info.time_ms));
        }

        if base.document_type_field_is_required(property_names::TRANSFERRED_AT_BLOCK_HEIGHT)? {
            modified_document.set_transferred_at_block_height(Some(block_info.height));
        }

        if base.document_type_field_is_required(property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT)? {
            modified_document.set_transferred_at_core_block_height(Some(block_info.core_height));
        }

        Ok(DocumentPurchaseTransitionActionV0 {
            base,
            document: modified_document,
            original_owner_id,
            price: *price,
        })
    }
}
