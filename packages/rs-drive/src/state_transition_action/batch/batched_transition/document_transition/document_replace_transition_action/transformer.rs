use dpp::block::block_info::BlockInfo;
use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::TimestampMillis;
use dpp::prelude::{BlockHeight, ConsensusValidationResult, CoreBlockHeight, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::DocumentReplaceTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0};

impl DocumentReplaceTransitionAction {
    /// try from borrowed
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_document_replace_transition(
        document_replace_transition: &DocumentReplaceTransition,
        owner_id: Identifier,
        originally_created_at: Option<TimestampMillis>,
        originally_created_at_block_height: Option<BlockHeight>,
        originally_created_at_core_block_height: Option<CoreBlockHeight>,
        originally_transferred_at: Option<TimestampMillis>,
        originally_transferred_at_block_height: Option<BlockHeight>,
        originally_transferred_at_core_block_height: Option<CoreBlockHeight>,
        original_creator_id: Option<Identifier>,
        block_info: &BlockInfo,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        match document_replace_transition {
            DocumentReplaceTransition::V0(v0) => {
                DocumentReplaceTransitionActionV0::try_from_borrowed_document_replace_transition(
                    v0,
                    owner_id,
                    originally_created_at,
                    originally_created_at_block_height,
                    originally_created_at_core_block_height,
                    originally_transferred_at,
                    originally_transferred_at_block_height,
                    originally_transferred_at_core_block_height,
                    original_creator_id,
                    block_info,
                    user_fee_increase,
                    get_data_contract,
                )
            }
        }
    }
}
