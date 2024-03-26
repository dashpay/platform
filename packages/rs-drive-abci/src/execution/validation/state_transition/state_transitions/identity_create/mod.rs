pub(crate) mod identity_and_signatures;
mod state;
mod basic_structure;
mod advanced_structure;
mod transform_into_partially_used_asset_lock_action;

use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::identity::PartialIdentity;

use crate::error::execution::ExecutionError;

use crate::execution::validation::state_transition::identity_create::state::v0::IdentityCreateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::identity_create::basic_structure::v0::IdentityCreateStateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::{StateTransitionBasicStructureValidationV0, StateTransitionStateValidationV0, StateTransitionStructureKnownInStateValidationV0};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform::{PlatformRef, PlatformStateRef};

use crate::rpc::core::CoreRPCLike;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::ValidationMode;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use crate::execution::validation::state_transition::identity_create::advanced_structure::v0::IdentityCreateStateTransitionAdvancedStructureValidationV0;

impl StateTransitionActionTransformerV0 for IdentityCreateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
        _validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform, execution_context, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityCreateTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .basic_structure
        {
            Some(0) => self.validate_base_structure_v0(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}


impl StateTransitionStructureKnownInStateValidationV0 for IdentityCreateTransition {
    fn validate_advanced_structure_from_state(
        &self,
        platform: &PlatformStateRef,
        action: &StateTransitionAction,
        _identity: Option<&PartialIdentity>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .advanced_structure
        {
            Some(0) => {
                let StateTransitionAction::IdentityCreateAction(identity_create_action) =
                    action
                    else {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "action must be a identity create transition action",
                        )));
                    };
                self.validate_advanced_structure_from_state_v0(
                    platform,
                    identity_create_action,
                    transaction,
                    platform_version,
                )
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_advanced_structure_from_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity create transition: validate_advanced_structure_from_state".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    fn has_advanced_structure_validation_with_state(&self) -> bool {
        true
    }

    fn requires_advanced_structure_validation_with_state_on_check_tx(&self) -> bool {
        false
    }
}

impl StateTransitionStateValidationV0 for IdentityCreateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .state
        {
            0 => {
                let action =
                    action.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "identity create validation should always an action",
                    )))?;
                let StateTransitionAction::IdentityCreateAction(_) =
                    &action
                    else {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "action must be a identity create transition action",
                        )));
                    };
                self.validate_state_v0(platform, action, execution_context, tx, platform_version)
            },
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
