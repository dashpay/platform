use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::StateTransitionLike;
use drive::state_transition_action::StateTransitionAction;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use drive::grovedb::TransactionArg;
use drive::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::documents_batch::data_triggers::DataTriggerExecutionContext;
use crate::execution::validation::state_transition::documents_batch::state::v0::data_triggers::execute_data_triggers;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::execution::validation::state_transition::documents_batch::state::v0::validate_documents_batch_transition_state::validate_document_batch_transition_state;

mod data_triggers;
pub mod fetch_documents;
pub mod validate_documents_batch_transition_state;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchStateTransitionStateValidationV0
{
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        action: DocumentsBatchTransitionAction,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionStateValidationV0 for DocumentsBatchTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        action: DocumentsBatchTransitionAction,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut state_transition_execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

        //todo: we have the action already created, we should use it instead of making another action

        let mut validation_result = validate_document_batch_transition_state(
            false,
            &platform.into(),
            self,
            tx,
            &mut state_transition_execution_context,
        )?;

        // Do not execute data triggers if there are already any state-based errors
        if !validation_result.is_valid_with_data() {
            return Ok(validation_result.map(Into::into));
        }

        let state_transition_action = validation_result.data.as_ref().unwrap();

        let data_trigger_execution_context = DataTriggerExecutionContext {
            platform: &platform.into(),
            transaction: tx,
            owner_id: &self.owner_id(),
            state_transition_execution_context: &state_transition_execution_context,
        };

        let data_triggers_validation_result = execute_data_triggers(
            state_transition_action.transitions(),
            &data_trigger_execution_context,
            platform.state.current_platform_version()?,
        )?;

        validation_result.add_errors_into(data_triggers_validation_result.errors);

        Ok(validation_result.map(Into::into))
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)?;
        let validation_result = validate_document_batch_transition_state(
            true,
            &platform.into(),
            self,
            tx,
            &mut execution_context,
        )?;
        Ok(validation_result.map(Into::into))
    }
}
