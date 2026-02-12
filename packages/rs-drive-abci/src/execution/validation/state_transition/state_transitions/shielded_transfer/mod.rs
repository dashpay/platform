mod transform_into_action;

#[cfg(test)]
mod tests;

use dpp::block::block_info::BlockInfo;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::shielded_transfer::transform_into_action::v0::ShieldedTransferStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::platform_types::platform_state::PlatformStateV0Methods;

/// A trait to transform into an action for shielded transfer transition
pub trait StateTransitionShieldedTransferTransitionActionTransformer {
    /// Transform into an action for shielded transfer transition
    fn transform_into_action_for_shielded_transfer_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionShieldedTransferTransitionActionTransformer for ShieldedTransferTransition {
    fn transform_into_action_for_shielded_transfer_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .shielded_transfer_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                platform.drive,
                tx,
                block_info,
                execution_context,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "shielded transfer transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
