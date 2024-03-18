pub(crate) mod identity_and_signatures;
mod state;
mod structure;

use crate::error::Error;
use dpp::block::block_info::BlockInfo;

use crate::error::execution::ExecutionError;

use crate::execution::validation::state_transition::identity_create::state::v0::IdentityCreateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::identity_create::structure::v0::IdentityCreateStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionBasicStructureValidationV0, StateTransitionStateValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform::PlatformRef;

use crate::rpc::core::CoreRPCLike;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::ValidationMode;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

impl StateTransitionActionTransformerV0 for IdentityCreateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
        _validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform, execution_context, platform_version),
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

impl StateTransitionStateValidationV0 for IdentityCreateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
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
            0 => self.validate_state_v0(platform, execution_context, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
