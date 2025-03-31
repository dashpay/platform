use dpp::block::block_info::BlockInfo;
use dpp::document::Document;
use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::DocumentPurchaseTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::{DocumentPurchaseTransitionAction, DocumentPurchaseTransitionActionV0};

impl DocumentPurchaseTransitionAction {
    /// try from borrowed
    pub fn try_from_borrowed_document_purchase_transition(
        document_purchase_transition: &DocumentPurchaseTransition,
        owner_id: Identifier,
        original_document: Document,
        purchaser_id: Identifier,
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
        match document_purchase_transition {
            DocumentPurchaseTransition::V0(v0) => {
                DocumentPurchaseTransitionActionV0::try_from_borrowed_document_purchase_transition(
                    v0,
                    owner_id,
                    original_document,
                    purchaser_id,
                    block_info,
                    user_fee_increase,
                    get_data_contract,
                )
            }
        }
    }
}
