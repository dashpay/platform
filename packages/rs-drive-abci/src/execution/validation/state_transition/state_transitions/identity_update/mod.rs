pub(crate) mod identity_and_signatures;
mod nonce;
mod state;
mod structure;

use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::platform_types::platform::{PlatformRef, PlatformStateRef};
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::identity_update::state::v0::IdentityUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::identity_update::structure::v0::IdentityUpdateStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionBasicStructureValidationV0, StateTransitionStateValidationV0,
    StateTransitionStructureKnownInStateValidationV0,
};

use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl StateTransitionActionTransformerV0 for IdentityUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _validate: bool,
        _execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityUpdateTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .base_structure
        {
            0 => self.validate_base_structure_v0(platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionStateValidationV0 for IdentityUpdateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
