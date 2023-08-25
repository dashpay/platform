mod action_validation;
mod base_structure;
mod data_triggers;
mod state;
mod transformer;

use dpp::prelude::*;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::{PlatformRef, PlatformStateRef};
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::documents_batch::base_structure::v0::DocumentsBatchStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::documents_batch::state::v0::DocumentsBatchStateTransitionStateValidationV0;

use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionStateValidationV0, StateTransitionStructureValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl StateTransitionActionTransformerV0 for DocumentsBatchTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        validate: bool,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(&platform.into(), validate, tx),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionStructureValidationV0 for DocumentsBatchTransition {
    fn validate_structure(
        &self,
        platform: &PlatformStateRef,
        action: Option<&StateTransitionAction>,
        protocol_version: u32,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .structure
        {
            0 => {
                let action =
                    action.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "documents batch structure validation should have an action",
                    )))?;
                let StateTransitionAction::DocumentsBatchAction(documents_batch_transition_action) = action else  {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution("action must be a documents batch transition action")));
                };
                self.validate_structure_v0(
                    platform,
                    documents_batch_transition_action,
                    platform_version,
                )
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: base structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionStateValidationV0 for DocumentsBatchTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .state
        {
            0 => {
                let action =
                    action.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "documents batch structure validation should have an action",
                    )))?;
                let StateTransitionAction::DocumentsBatchAction(documents_batch_transition_action) = action else  {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution("action must be a documents batch transition action")));
                };
                self.validate_state_v0(
                    documents_batch_transition_action,
                    &platform.into(),
                    tx,
                    platform_version,
                )
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
