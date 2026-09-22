mod nonce;
#[cfg(test)]
mod tests;
mod transform_into_action;

use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::shield_from_identity::transform_into_action::v0::ShieldFromIdentityStateTransitionTransformIntoActionValidationV0;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::check_tx_proof_verifier::CheckTxProofVerifier;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

/// Transform a `ShieldFromIdentity` transition into its action, verifying the Orchard proof
/// inside the transform (like `ShieldFromAssetLock`) so a failed proof becomes a paid,
/// nonce-consuming penalty on the funding identity instead of a free rejection.
///
/// `check_tx_proof_verifier` is the node-local CheckTx admission budget: CheckTx passes
/// `Some` so the expensive verification runs only after the cheap current-state checks
/// passed and only under a permit; proposal and block processing pass `None`.
pub(in crate::execution) trait StateTransitionShieldFromIdentityTransitionActionTransformer {
    /// Transform into an action for the shield from identity transition
    fn transform_into_action_for_shield_from_identity_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        check_tx_proof_verifier: Option<&CheckTxProofVerifier>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionShieldFromIdentityTransitionActionTransformer for ShieldFromIdentityTransition {
    fn transform_into_action_for_shield_from_identity_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        check_tx_proof_verifier: Option<&CheckTxProofVerifier>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .shield_from_identity_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                platform.drive,
                tx,
                block_info,
                execution_context,
                check_tx_proof_verifier,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "shield from identity transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionActionTransformer for ShieldFromIdentityTransition {
    /// Proposal and block processing entry point: no CheckTx proof budget applies.
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        _remaining_address_input_balances: &Option<
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        >,
        _validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        self.transform_into_action_for_shield_from_identity_transition(
            platform,
            block_info,
            execution_context,
            None,
            tx,
        )
    }
}
