mod transform_into_action;

use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::token_purchase_from_shielded_pool::transform_into_action::v0::TokenPurchaseFromShieldedPoolStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

/// A trait to transform into an action for the token purchase from shielded pool transition
pub trait StateTransitionTokenPurchaseFromShieldedPoolTransitionActionTransformer {
    /// Transform into an action for the token purchase from shielded pool transition
    fn transform_into_action_for_token_purchase_from_shielded_pool_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionTokenPurchaseFromShieldedPoolTransitionActionTransformer
    for TokenPurchaseFromShieldedPoolTransition
{
    fn transform_into_action_for_token_purchase_from_shielded_pool_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .token_purchase_from_shielded_pool_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform.drive, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "token purchase from shielded pool transition: transform_into_action"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
