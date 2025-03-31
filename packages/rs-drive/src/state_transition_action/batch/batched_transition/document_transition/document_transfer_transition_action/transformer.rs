use dpp::block::block_info::BlockInfo;
use dpp::document::Document;
use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::DocumentTransferTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::{DocumentTransferTransitionAction, DocumentTransferTransitionActionV0};

impl DocumentTransferTransitionAction {
    /// try from borrowed
    pub fn try_from_borrowed_document_transfer_transition(
        document_transfer_transition: &DocumentTransferTransition,
        owner_id: Identifier,
        original_document: Document,
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
        match document_transfer_transition {
            DocumentTransferTransition::V0(v0) => {
                DocumentTransferTransitionActionV0::try_from_borrowed_document_transfer_transition(
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
