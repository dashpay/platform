mod transform_into_action;

#[cfg(test)]
mod tests;

use dpp::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::identity_top_up_from_shielded_pool::transform_into_action::v0::IdentityTopUpFromShieldedPoolStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

/// A trait to transform into an action for the identity top up from shielded pool transition
pub trait StateTransitionIdentityTopUpFromShieldedPoolTransitionActionTransformer {
    /// Transform into an action for the identity top up from shielded pool transition
    fn transform_into_action_for_identity_top_up_from_shielded_pool_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionIdentityTopUpFromShieldedPoolTransitionActionTransformer
    for IdentityTopUpFromShieldedPoolTransition
{
    fn transform_into_action_for_identity_top_up_from_shielded_pool_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_top_up_from_shielded_pool_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform.drive, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity top up from shielded pool transition: transform_into_action"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
