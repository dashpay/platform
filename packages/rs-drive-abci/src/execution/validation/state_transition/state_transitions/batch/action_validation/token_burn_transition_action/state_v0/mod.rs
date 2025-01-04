use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use dpp::consensus::state::document::document_contest_currently_locked_error::DocumentContestCurrentlyLockedError;
use dpp::consensus::state::document::document_contest_identity_already_contestant::DocumentContestIdentityAlreadyContestantError;
use dpp::consensus::state::document::document_contest_not_joinable_error::DocumentContestNotJoinableError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{IdentityDoesNotHaveEnoughTokenBalanceError, UnauthorizedTokenActionError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::multi_identity_events::ActionTaker;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::token_burn_transition_action::{TokenBurnTransitionAction, TokenBurnTransitionActionAccessorsV0};
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

pub(super) trait TokenBurnTransitionActionStateValidationV0 {
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
impl TokenBurnTransitionActionStateValidationV0 for TokenBurnTransitionAction {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // Let's first check to see if we are authorized to perform this action
        let contract = &self.data_contract_fetch_info_ref().contract;
        let token_configuration = contract.expected_token_configuration(self.token_position())?;
        let rules = token_configuration.manual_burning_rules();
        let main_control_group = token_configuration
            .main_control_group()
            .map(|position| contract.expected_group(position))
            .transpose()?;

        if !rules.can_make_change(
            &contract.owner_id(),
            main_control_group,
            contract.groups(),
            &ActionTaker::SingleIdentity(owner_id),
        ) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                    UnauthorizedTokenActionError::new(
                        owner_id,
                        "burn".to_string(),
                        rules.authorized_to_make_change_action_takers().clone(),
                    ),
                )),
            ));
        }

        // We need to verify that we have enough of the token
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                self.token_id().to_buffer(),
                owner_id.to_buffer(),
                transaction,
                platform_version,
            )?
            .unwrap_or_default();
        execution_context.add_operation(ValidationOperation::RetrieveIdentityTokenBalance);
        if balance < self.burn_amount() {
            // The identity does not exist
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::IdentityDoesNotHaveEnoughTokenBalanceError(
                    IdentityDoesNotHaveEnoughTokenBalanceError::new(
                        owner_id,
                        self.burn_amount(),
                        balance,
                        "burn".to_string(),
                    ),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
