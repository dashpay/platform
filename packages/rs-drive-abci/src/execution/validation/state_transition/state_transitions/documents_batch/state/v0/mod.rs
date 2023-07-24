use dpp::document::DocumentsBatchTransition;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::state_transition::StateTransitionAction;
use dpp::state_transition_action::StateTransitionAction;
use drive::grovedb::TransactionArg;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::execution::validation::state_transition::documents_batch::state::v0::validate_documents_batch_transition_state::validate_document_batch_transition_state;

pub mod execute_data_triggers;
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
        let validation_result = validate_document_batch_transition_state(
            false,
            &platform.into(),
            self,
            tx,
            &StateTransitionExecutionContext::default(),
        )?;
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
