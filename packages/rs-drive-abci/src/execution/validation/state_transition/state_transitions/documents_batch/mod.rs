mod action_validation;
mod advanced_structure;
mod data_triggers;
mod identity_contract_nonce;
mod state;
mod transformer;

use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::identity::PartialIdentity;
use dpp::prelude::*;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::platform_types::platform::{PlatformRef, PlatformStateRef};
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::documents_batch::advanced_structure::v0::DocumentsBatchStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::documents_batch::identity_contract_nonce::v0::DocumentsBatchStateTransitionIdentityContractNonceV0;
use crate::execution::validation::state_transition::documents_batch::state::v0::DocumentsBatchStateTransitionStateValidationV0;

use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionBasicStructureValidationV0, StateTransitionNonceValidationV0,
    StateTransitionStateValidationV0, StateTransitionStructureKnownInStateValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;

impl ValidationMode {
    /// Returns a bool on whether we should validate that documents are valid against the state
    pub fn should_validate_document_valid_against_state(&self) -> bool {
        match self {
            ValidationMode::CheckTx => false,
            ValidationMode::RecheckTx => false,
            ValidationMode::Validator => true,
            ValidationMode::NoValidation => false,
        }
    }
}

impl StateTransitionActionTransformerV0 for DocumentsBatchTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(&platform.into(), block_info, validation_mode, tx),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for DocumentsBatchTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .basic_structure
        {
            0 => {
                // There is nothing expensive here
                self.validate_base_structure(platform_version)
                    .map_err(Error::Protocol)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: base structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionNonceValidationV0 for DocumentsBatchTransition {
    fn validate_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .revision
        {
            0 => self.validate_identity_contract_nonces_v0(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: revision".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionStructureKnownInStateValidationV0 for DocumentsBatchTransition {
    fn validate_advanced_structure_from_state(
        &self,
        action: &StateTransitionAction,
        identity: Option<&PartialIdentity>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .advanced_structure
        {
            0 => {
                let identity =
                    identity.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "The identity must be known on advanced structure validation",
                    )))?;
                let StateTransitionAction::DocumentsBatchAction(documents_batch_transition_action) =
                    action
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "action must be a documents batch transition action",
                    )));
                };
                self.validate_advanced_structure_from_state_v0(
                    documents_batch_transition_action,
                    identity,
                    execution_context,
                    platform_version,
                )
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: advanced structure from state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn has_advanced_structure_validation_with_state(&self) -> bool {
        true
    }

    fn requires_advanced_structure_validation_with_state_on_check_tx(&self) -> bool {
        true
    }
}

impl StateTransitionStateValidationV0 for DocumentsBatchTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _validation_mode: ValidationMode,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

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
                let StateTransitionAction::DocumentsBatchAction(documents_batch_transition_action) =
                    action
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "action must be a documents batch transition action",
                    )));
                };
                self.validate_state_v0(
                    documents_batch_transition_action,
                    &platform.into(),
                    epoch,
                    execution_context,
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
