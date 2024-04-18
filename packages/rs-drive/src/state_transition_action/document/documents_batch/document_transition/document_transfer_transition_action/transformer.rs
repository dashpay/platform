use dpp::block::block_info::BlockInfo;
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::identity::TimestampMillis;
use dpp::prelude::{BlockHeight, CoreBlockHeight};
use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransferTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_transfer_transition_action::{DocumentTransferTransitionAction, DocumentTransferTransitionActionV0};

impl DocumentTransferTransitionAction {
    /// try from borrowed
    pub fn try_from_borrowed_document_transfer_transition(
        document_transfer_transition: &DocumentTransferTransition,
        originally_created_at: Option<TimestampMillis>,
        originally_created_at_block_height: Option<BlockHeight>,
        originally_created_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match document_transfer_transition {
            DocumentTransferTransition::V0(v0) => Ok(
                DocumentTransferTransitionActionV0::try_from_borrowed_document_transfer_transition(
                    v0,
                    originally_created_at,
                    originally_created_at_block_height,
                    originally_created_at_core_block_height,
                    block_info,
                    get_data_contract,
                )?
                .into(),
            ),
        }
    }
}
