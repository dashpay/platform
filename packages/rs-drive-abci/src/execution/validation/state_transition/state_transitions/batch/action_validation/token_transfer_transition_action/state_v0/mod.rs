use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use dpp::consensus::state::document::document_contest_currently_locked_error::DocumentContestCurrentlyLockedError;
use dpp::consensus::state::document::document_contest_identity_already_contestant::DocumentContestIdentityAlreadyContestantError;
use dpp::consensus::state::document::document_contest_not_joinable_error::DocumentContestNotJoinableError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
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
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::state::v0::fetch_contender::fetch_contender;
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::fetch_document_with_id;
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
