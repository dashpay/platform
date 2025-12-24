use dpp::block::block_info::BlockInfo;
use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
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
        original_document: &Document,
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
                    original_document,
                    block_info,
                    user_fee_increase,
                    get_data_contract,
                )
            }
        }
    }
}
