use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::DocumentDeleteTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::{DocumentDeleteTransitionAction, DocumentDeleteTransitionActionV0};

impl DocumentDeleteTransitionAction {
    /// from borrowed
    pub fn try_from_document_borrowed_delete_transition_with_contract_lookup(
        value: &DocumentDeleteTransition,
        owner_id: Identifier,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        match value {
            DocumentDeleteTransition::V0(v0) => DocumentDeleteTransitionActionV0::try_from_borrowed_document_delete_transition_with_contract_lookup(v0, owner_id, user_fee_increase,  get_data_contract),
        }
    }
}
