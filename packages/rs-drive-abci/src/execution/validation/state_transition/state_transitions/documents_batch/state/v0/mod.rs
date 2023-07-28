use dpp::document::DocumentsBatchTransition;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::state_transition::{StateTransitionAction, StateTransitionLike};
use dpp::state_transition_action::StateTransitionAction;
use dpp::state_transition_action::StateTransitionAction;
use drive::grovedb::TransactionArg;
use crate::error::Error;
use crate::execution::validation::state_transition::documents_batch::data_triggers::DataTriggerExecutionContext;
use crate::execution::validation::state_transition::documents_batch::state::v0::data_triggers::execute_data_triggers;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::execution::validation::state_transition::documents_batch::state::v0::validate_documents_batch_transition_state::validate_document_batch_transition_state;

mod data_triggers;
pub mod fetch_documents;
pub mod validate_documents_batch_transition_state;

pub(crate) trait StateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStateValidationV0 for DocumentsBatchTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let state_transition_execution_context = StateTransitionExecutionContext::default();

        let mut validation_result = validate_document_batch_transition_state(
            false,
            &platform.into(),
            self,
            tx,
            state_transition_execution_context,
        )?;

        // Do not execute data triggers if there are already any state-based errors
        if !validation_result.is_valid() {
            return Ok(validation_result.map(Into::into));
        }

        let data_trigger_execution_context = DataTriggerExecutionContext {
            platform,
            transaction,
            owner_id: self.get_owner_id(),
            data_contract,
            state_transition_execution_context,
        };

        let document_transition_actions = validation_result.into_data()?;

        let data_triggers_validation_result = execute_data_triggers(
            document_transition_actions.transitions(),
            &data_trigger_execution_context,
            platform.state.current_platform_version()?,
        )?;

        validation_result.add_errors(data_triggers_validation_result.errors());

        Ok(validation_result.map(Into::into))
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let validation_result = validate_document_batch_transition_state(
            true,
            &platform.into(),
            self,
            tx,
            &StateTransitionExecutionContext::default(),
        )?;
        Ok(validation_result.map(Into::into))
    }
}
