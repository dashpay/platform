#[cfg(test)]
mod tests;
mod transform_into_action;

use dpp::block::block_info::BlockInfo;
use dpp::state_transition::state_transitions::shielded::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::shielded_withdrawal::transform_into_action::v0::ShieldedWithdrawalStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

/// A trait to transform into an action for shielded withdrawal transition
pub trait StateTransitionShieldedWithdrawalTransitionActionTransformer {
    /// Transform into an action for shielded withdrawal transition
    fn transform_into_action_for_shielded_withdrawal_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionShieldedWithdrawalTransitionActionTransformer for ShieldedWithdrawalTransition {
    fn transform_into_action_for_shielded_withdrawal_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .shielded_withdrawal_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform.drive, block_info, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "shielded withdrawal transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
