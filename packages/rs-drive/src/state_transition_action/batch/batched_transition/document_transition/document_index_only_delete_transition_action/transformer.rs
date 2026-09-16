use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::DocumentIndexOnlyDeleteTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::{DocumentIndexOnlyDeleteTransitionAction, DocumentIndexOnlyDeleteTransitionActionV0};

impl DocumentIndexOnlyDeleteTransitionAction {
    /// from borrowed
    pub fn try_from_document_borrowed_index_only_delete_transition_with_contract_lookup(
        value: &DocumentIndexOnlyDeleteTransition,
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
            DocumentIndexOnlyDeleteTransition::V0(v0) => DocumentIndexOnlyDeleteTransitionActionV0::try_from_borrowed_document_index_only_delete_transition_with_contract_lookup(v0, owner_id, user_fee_increase, get_data_contract),
        }
    }
}
