use dpp::block::block_info::BlockInfo;
use dpp::document::Document;
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentUpdatePriceTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_update_price_transition_action::{DocumentUpdatePriceTransitionAction, DocumentUpdatePriceTransitionActionV0};

impl DocumentUpdatePriceTransitionAction {
    /// try from borrowed
    pub fn try_from_borrowed_document_update_price_transition(
        document_update_price_transition: &DocumentUpdatePriceTransition,
        original_document: Document,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match document_update_price_transition {
            DocumentUpdatePriceTransition::V0(v0) => Ok(
                DocumentUpdatePriceTransitionActionV0::try_from_borrowed_document_update_price_transition(
                    v0,
                    original_document,
                    block_info,
                    get_data_contract,
                )?
                .into(),
            ),
        }
    }
}
