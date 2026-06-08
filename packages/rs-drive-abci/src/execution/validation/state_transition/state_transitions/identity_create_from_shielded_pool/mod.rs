mod transform_into_action;

#[cfg(test)]
mod tests;

use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_create_from_shielded_pool::transform_into_action::v0::IdentityCreateFromShieldedPoolStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

/// A trait to transform into an action for the identity-create-from-shielded-pool transition.
pub trait StateTransitionIdentityCreateFromShieldedPoolTransitionActionTransformer {
    /// Transform into an action.
    ///
    /// The key structure + per-key proof-of-possession are *verified* earlier (in
    /// `validate_shielded_proof`, before Halo 2). This does the stateful pool checks and records the
    /// per-key signature-verification operations into the execution context for fee accounting (no
    /// `signable_bytes` are needed — the verification already happened).
    fn transform_into_action_for_identity_create_from_shielded_pool_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionIdentityCreateFromShieldedPoolTransitionActionTransformer
    for IdentityCreateFromShieldedPoolTransition
{
    fn transform_into_action_for_identity_create_from_shielded_pool_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_from_shielded_pool_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                platform.drive,
                execution_context,
                tx,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create from shielded pool transition: transform_into_action"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
