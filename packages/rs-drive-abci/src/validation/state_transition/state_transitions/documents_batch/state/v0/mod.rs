use dpp::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::state_transition::data_contract_create_transition::{DataContractCreateTransition, DataContractCreateTransitionAction};
use dpp::document::DocumentsBatchTransition;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::state_transition::StateTransitionAction;
use drive::grovedb::TransactionArg;
use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::document_state_validation::validate_documents_batch_transition_state::validate_document_batch_transition_state;
use crate::validation::state_transition::processor::v0::StateTransitionValidationV0;

pub(in crate::validation::state_transition) trait StateTransitionStateValidationV0 {
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
