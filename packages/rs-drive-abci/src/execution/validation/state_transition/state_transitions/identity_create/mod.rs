pub(crate) mod identity_and_signatures;
mod state;
mod structure;

use crate::error::Error;

use crate::error::execution::ExecutionError;

use crate::execution::validation::state_transition::identity_create::state::v0::IdentityCreateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::identity_create::structure::v0::IdentityCreateStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionBasicStructureValidationV0, StateTransitionStateValidationV0,
    StateTransitionStructureKnownInStateValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform::{PlatformRef, PlatformStateRef};

use crate::rpc::core::CoreRPCLike;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

impl StateTransitionActionTransformerV0 for IdentityCreateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _validate: bool,
        execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform
            .version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform, execution_context, platform.version),
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
            .base_structure
        {
            0 => self.validate_base_structure_v0(platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionStateValidationV0 for IdentityCreateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform
            .version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, execution_context, tx, platform.version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
