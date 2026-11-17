use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::{DocumentEraseTransitionAction, DocumentEraseTransitionActionV0};
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::state_transition::batch_transition::batched_transition::DocumentEraseTransition;
use dpp::ProtocolError;
use std::sync::Arc;

impl DocumentEraseTransitionAction {
    /// from borrowed
    pub fn try_from_document_borrowed_erase_transition_with_contract_lookup(
        value: &DocumentEraseTransition,
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
            DocumentEraseTransition::V0(v0) => {
                DocumentEraseTransitionActionV0::try_from_borrowed_document_erase_transition_with_contract_lookup(
                    v0,
                    owner_id,
                    user_fee_increase,
                    get_data_contract,
                )
            }
        }
    }
}
