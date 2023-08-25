use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::StateTransitionLike;
use drive::state_transition_action::StateTransitionAction;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use drive::grovedb::TransactionArg;
use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use drive::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_create_transition_action::DocumentCreateTransitionActionValidation;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_delete_transition_action::DocumentDeleteTransitionActionValidation;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
use crate::execution::validation::state_transition::documents_batch::data_triggers::DataTriggerExecutionContext;
use crate::execution::validation::state_transition::documents_batch::state::v0::data_triggers::execute_data_triggers;
use crate::platform_types::platform::{PlatformRef, PlatformStateRef};
use crate::execution::validation::state_transition::state_transitions::documents_batch::transformer::v0::DocumentsBatchTransitionTransformerV0;
use crate::rpc::core::CoreRPCLike;
mod data_triggers;
pub mod fetch_documents;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchStateTransitionStateValidationV0
{
    fn validate_state_v0(
        &self,
        action: DocumentsBatchTransitionAction,
        platform: &PlatformStateRef,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
        platform: &PlatformStateRef,
        validate: bool,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionStateValidationV0 for DocumentsBatchTransition {
    fn validate_state_v0(
        &self,
        state_transition_action: DocumentsBatchTransitionAction,
        platform: &PlatformStateRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::new();

        let mut state_transition_execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

        let owner_id = state_transition_action.owner_id();

        // Next we need to validate the structure of all actions (this means with the data contract)
        for transition in state_transition_action.transitions() {
            match transition {
                DocumentTransitionAction::CreateAction(create_action) => {
                    let result = create_action.validate_state(
                        platform,
                        owner_id,
                        transaction,
                        platform_version,
                    )?;
                    if !result.is_valid() {
                        validation_result.add_errors(result.errors);
                        return Ok(validation_result);
                    }
                }
                DocumentTransitionAction::ReplaceAction(replace_action) => {
                    let result = replace_action.validate_state(
                        platform,
                        owner_id,
                        transaction,
                        platform_version,
                    )?;
                    if !result.is_valid() {
                        validation_result.add_errors(result.errors);
                        return Ok(validation_result);
                    }
                }
                DocumentTransitionAction::DeleteAction(delete_action) => {
                    let result = delete_action.validate_state(
                        platform,
                        owner_id,
                        transaction,
                        platform_version,
                    )?;
                    if !result.is_valid() {
                        validation_result.add_errors(result.errors);
                        return Ok(validation_result);
                    }
                }
            }
        }

        let data_trigger_execution_context = DataTriggerExecutionContext {
            platform,
            transaction,
            owner_id: &self.owner_id(),
            state_transition_execution_context: &state_transition_execution_context,
        };

        let data_triggers_validation_result = execute_data_triggers(
            state_transition_action.transitions(),
            &data_trigger_execution_context,
            platform.state.current_platform_version()?,
        )?;

        validation_result.add_errors_into(data_triggers_validation_result.errors);

        validation_result.set_data(state_transition_action.into());

        Ok(validation_result)
    }

    fn transform_into_action_v0(
        &self,
        platform: &PlatformStateRef,
        validate: bool,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)?;
        let validation_result =
            self.try_into_action_v0(platform, validate, tx, &mut execution_context)?;
        Ok(validation_result.map(Into::into))
    }
}
