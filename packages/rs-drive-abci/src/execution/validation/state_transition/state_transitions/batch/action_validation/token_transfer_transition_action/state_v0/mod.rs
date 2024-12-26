use dpp::block::block_info::BlockInfo;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::{TokenTransferTransitionAction};
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters};
use drive::error::drive::DriveError;
use drive::query::TransactionArg;
use drive::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformStateRef;

pub(super) trait TokenTransferTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenTransferTransitionActionStateValidationV0 for TokenTransferTransitionAction {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // todo verify that minting would not break max supply

        Ok(SimpleConsensusValidationResult::new())
    }
}
