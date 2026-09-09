mod transform_into_action;

#[cfg(test)]
mod tests;

use dpp::block::block_info::BlockInfo;
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::shield_from_asset_lock::transform_into_action::v0::ShieldFromAssetLockStateTransitionTransformIntoActionValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::check_tx_proof_verifier::CheckTxProofVerifier;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

/// A trait to transform into an action for shield from asset lock transition
pub(in crate::execution) trait StateTransitionShieldFromAssetLockTransitionActionTransformer {
    /// Transform into an action for shield from asset lock transition
    #[allow(clippy::too_many_arguments)]
    fn transform_into_action_for_shield_from_asset_lock_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        check_tx_proof_verifier: Option<&CheckTxProofVerifier>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionShieldFromAssetLockTransitionActionTransformer
    for ShieldFromAssetLockTransition
{
    #[allow(clippy::too_many_arguments)]
    fn transform_into_action_for_shield_from_asset_lock_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        _block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        check_tx_proof_verifier: Option<&CheckTxProofVerifier>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .shield_from_asset_lock_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                platform,
                signable_bytes,
                validation_mode,
                execution_context,
                check_tx_proof_verifier,
                tx,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "shield from asset lock transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
