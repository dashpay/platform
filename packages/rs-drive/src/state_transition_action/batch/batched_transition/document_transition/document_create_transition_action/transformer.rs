use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use grovedb::TransactionArg;
use std::sync::Arc;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionV0};

impl DocumentCreateTransitionAction {
    /// from_document_borrowed_create_transition_with_contract_lookup
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_document_borrowed_create_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        transaction: TransactionArg,
        value: &DocumentCreateTransition,
        block_info: &BlockInfo,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        match value {
            DocumentCreateTransition::V0(v0) => {
                DocumentCreateTransitionActionV0::try_from_borrowed_document_create_transition_with_contract_lookup(drive, owner_id, transaction, v0, block_info, user_fee_increase, get_data_contract, platform_version)
            }
        }
    }
}
