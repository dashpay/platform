use dpp::block::block_info::BlockInfo;
use dpp::document::Document;
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentPurchaseTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_purchase_transition_action::{DocumentPurchaseTransitionAction, DocumentPurchaseTransitionActionV0};

impl DocumentPurchaseTransitionAction {
    /// try from borrowed
    pub fn try_from_borrowed_document_purchase_transition(
        document_purchase_transition: &DocumentPurchaseTransition,
        original_document: Document,
        purchaser_id: Identifier,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match document_purchase_transition {
            DocumentPurchaseTransition::V0(v0) => Ok(
                DocumentPurchaseTransitionActionV0::try_from_borrowed_document_purchase_transition(
                    v0,
                    original_document,
                    purchaser_id,
                    block_info,
                    get_data_contract,
                )?
                .into(),
            ),
        }
    }
}
