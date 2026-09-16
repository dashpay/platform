mod transform_into_action;

use dpp::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::token_unshield_with_shielded_fee::transform_into_action::v0::TokenUnshieldWithShieldedFeeStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

/// A trait to transform into an action for the token unshield with shielded fee transition
pub trait StateTransitionTokenUnshieldWithShieldedFeeTransitionActionTransformer {
    /// Transform into an action for the token unshield with shielded fee transition
    fn transform_into_action_for_token_unshield_with_shielded_fee_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionTokenUnshieldWithShieldedFeeTransitionActionTransformer
    for TokenUnshieldWithShieldedFeeTransition
{
    fn transform_into_action_for_token_unshield_with_shielded_fee_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .token_unshield_with_shielded_fee_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform.drive, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "token unshield with shielded fee transition: transform_into_action"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
