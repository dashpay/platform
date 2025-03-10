use dpp::block::block_info::BlockInfo;
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::identity::TimestampMillis;
use dpp::prelude::{BlockHeight, CoreBlockHeight};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::DocumentReplaceTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0};

impl DocumentReplaceTransitionAction {
    /// try from borrowed
    pub fn try_from_borrowed_document_replace_transition(
        document_replace_transition: &DocumentReplaceTransition,
        originally_created_at: Option<TimestampMillis>,
        originally_created_at_block_height: Option<BlockHeight>,
        originally_created_at_core_block_height: Option<CoreBlockHeight>,
        originally_transferred_at: Option<TimestampMillis>,
        originally_transferred_at_block_height: Option<BlockHeight>,
        originally_transferred_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition {
            DocumentReplaceTransition::V0(v0) => Ok(
                DocumentReplaceTransitionActionV0::try_from_borrowed_document_replace_transition(
                    v0,
                    originally_created_at,
                    originally_created_at_block_height,
                    originally_created_at_core_block_height,
                    originally_transferred_at,
                    originally_transferred_at_block_height,
                    originally_transferred_at_core_block_height,
                    block_info,
                    get_data_contract,
                )?
                .into(),
            ),
        }
    }
}
