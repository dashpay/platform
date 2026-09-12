use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, reconstruct_and_verify_bundle, FLAGS_OUTPUTS_ONLY,
};
use crate::platform_types::check_tx_proof_verifier::CheckTxProofVerifier;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shield_from_identity::ShieldFromIdentityTransitionAction;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::shield_from_identity) trait ShieldFromIdentityStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        check_tx_proof_verifier: Option<&CheckTxProofVerifier>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldFromIdentityStateTransitionTransformIntoActionValidationV0
    for ShieldFromIdentityTransition
{
    /// The identity signature, nonce, balance floor, and structure have all been checked
    /// by the processor before this point. The identity is debited exactly `amount`, so
    /// unlike `Shield` there is no per-input slack to reallocate; the pool total is read
    /// so the action can bump it, and that read is metered into the execution context so
    /// the identity pays for it.
    ///
    /// The Orchard proof is verified HERE rather than in the shared stateless proof step
    /// (like `ShieldFromAssetLock`), because a failed proof must not be a free rejection:
    /// the transition is identity-signed, so a rejection that left the nonce and balance
    /// untouched would let a funded identity submit invalid proofs indefinitely at no
    /// cost. A failed proof therefore returns a `BumpIdentityNonceAction` with the
    /// consensus error, which executes as a paid failure: the identity nonce is consumed
    /// and the identity is charged the versioned `shielded_proof_verification_failure`
    /// penalty on top of the processing metered so far. CheckTx never charges; it only
    /// admits the verification under `check_tx_proof_verifier`'s permit after the cheap
    /// checks passed, and rejects the transition on failure.
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        check_tx_proof_verifier: Option<&CheckTxProofVerifier>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

        let pool_read_fee = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(pool_read_fee));

        let ShieldFromIdentityTransition::V0(v0) = self;

        // CheckTx admits the expensive proof only after the signature, nonce, balance
        // floor, and pool read have passed, and only under its node-local budget.
        // Proposal and block processing pass `None`.
        let _check_tx_permit = match check_tx_proof_verifier {
            Some(verifier) => Some(verifier.try_acquire(v0.actions.len()).ok_or(
                Error::Execution(ExecutionError::CheckTxProofVerificationBusy),
            )?),
            None => None,
        };

        // Outputs-only bundle entering the pool, exactly like `Shield`. The identity ECDSA
        // signature already binds every bundle field to the identity and nonce, so no extra
        // sighash data is needed.
        if let Err(e) = reconstruct_and_verify_bundle(
            &v0.actions,
            FLAGS_OUTPUTS_ONLY,
            -(v0.amount as i64),
            &v0.anchor,
            v0.proof.as_slice(),
            &v0.binding_signature,
            &[],
        ) {
            // Paid penalty: the same versioned amount `ShieldFromAssetLock` burns from its
            // asset lock, charged here as a processing operation of the identity-paid
            // failure (the balance pre-check guarantees the identity can cover it).
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .shielded_proof_verification_failure;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(
                FeeResult {
                    processing_fee: penalty,
                    ..Default::default()
                },
            ));

            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_shield_from_identity_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![StateError::InvalidShieldedProofError(e).into()],
            ));
        }

        let result =
            ShieldFromIdentityTransitionAction::try_from_transition(self, current_total_balance);

        Ok(result.map(|action| action.into()))
    }
}
