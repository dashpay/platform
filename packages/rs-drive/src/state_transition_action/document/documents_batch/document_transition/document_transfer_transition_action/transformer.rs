use dpp::block::block_info::BlockInfo;
use dpp::document::Document;
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransferTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_transfer_transition_action::{DocumentTransferTransitionAction, DocumentTransferTransitionActionV0};

impl DocumentTransferTransitionAction {
    /// try from borrowed
    pub fn try_from_borrowed_document_transfer_transition(
        document_transfer_transition: &DocumentTransferTransition,
        original_document: Document,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match document_transfer_transition {
            DocumentTransferTransition::V0(v0) => Ok(
                DocumentTransferTransitionActionV0::try_from_borrowed_document_transfer_transition(
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
