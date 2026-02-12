use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, reconstruct_and_verify_bundle, validate_anchor_exists,
    validate_minimum_pool_notes, validate_nullifiers,
};
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shielded_withdrawal::ShieldedWithdrawalTransitionAction;
use drive::state_transition_action::system::penalize_shielded_pool_action::v0::PenalizeShieldedPoolActionV0;
use drive::state_transition_action::system::penalize_shielded_pool_action::PenalizeShieldedPoolAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::shielded_withdrawal) trait ShieldedWithdrawalStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldedWithdrawalStateTransitionTransformIntoActionValidationV0
    for ShieldedWithdrawalTransition
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Extract nullifiers, note commitments, and encrypted notes from serialized actions
        let nullifiers: Vec<[u8; 32]> = match self {
            ShieldedWithdrawalTransition::V0(v0) => {
                v0.actions.iter().map(|a| a.nullifier).collect()
            }
        };
        let note_commitments: Vec<[u8; 32]> = match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.actions.iter().map(|a| a.cmx).collect(),
        };
        let encrypted_notes: Vec<Vec<u8>> = match self {
            ShieldedWithdrawalTransition::V0(v0) => v0
                .actions
                .iter()
                .map(|a| a.encrypted_note.clone())
                .collect(),
        };

        // The anchor from the transition (Merkle root of commitment tree)
        let anchor: [u8; 32] = match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.anchor,
        };

        // Read current shielded pool total balance from GroveDB
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

        // Check minimum notes threshold for outgoing transitions (anonymity set)
        if let Some(err) = validate_minimum_pool_notes(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(err);
        }

        // Verify the pool has sufficient balance for the withdrawal.
        // value_balance is the total amount leaving the pool (amount + fee).
        let value_balance = match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.value_balance,
        };

        // value_balance > 0 is already validated in structure validation,
        // so the cast to u64 is safe here.
        if current_total_balance < (value_balance as u64) {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(
                    dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError::new(
                        format!(
                            "shielded pool has insufficient balance: pool has {} but withdrawal requires {}",
                            current_total_balance, value_balance
                        ),
                    ),
                )
                .into(),
            ));
        }

        // Verify the anchor exists in the recorded anchors tree
        if let Some(err) = validate_anchor_exists(
            drive,
            &anchor,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(err);
        }

        // Validate nullifiers: intra-bundle duplicates + already-spent in state
        if let Some(err) = validate_nullifiers(
            drive,
            &nullifiers,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(err);
        }

        // Calculate fees from the GroveDB operations
        let fee = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        // Verify the Orchard ZK proof, binding transparent fields to the sighash.
        // For shielded withdrawal, the extra_sighash_data binds the withdrawal
        // destination (output_script) and amount, preventing an attacker from
        // substituting a different output_script or amount while reusing a valid
        // Orchard bundle.
        let (
            st_actions,
            st_flags,
            st_value_balance,
            st_proof,
            st_binding_sig,
            output_script,
            amount,
        ) = match self {
            ShieldedWithdrawalTransition::V0(v0) => (
                &v0.actions,
                v0.flags,
                v0.value_balance,
                v0.proof.as_slice(),
                &v0.binding_signature,
                &v0.output_script,
                v0.amount,
            ),
        };

        // Serialize transparent fields to bind them to the Orchard sighash.
        // output_script.as_bytes() || amount.to_le_bytes()
        let mut extra_sighash_data = output_script.as_bytes().to_vec();
        extra_sighash_data.extend_from_slice(&amount.to_le_bytes());

        if let Err(e) = reconstruct_and_verify_bundle(
            st_actions,
            st_flags,
            st_value_balance,
            &anchor,
            st_proof,
            st_binding_sig,
            &extra_sighash_data,
        ) {
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .shielded_proof_verification_failure;

            let penalty_amount = std::cmp::min(penalty, current_total_balance);

            let penalize_action = StateTransitionAction::PenalizeShieldedPoolAction(
                PenalizeShieldedPoolAction::from(PenalizeShieldedPoolActionV0 {
                    penalty_amount,
                    nullifiers: nullifiers.clone(),
                    current_total_balance,
                }),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                penalize_action,
                vec![StateError::InvalidShieldedProofError(e).into()],
            ));
        }

        // Build the action, which includes creating the withdrawal document
        let creation_time_ms = block_info.time_ms;

        let result = ShieldedWithdrawalTransitionAction::try_from_transition(
            self,
            nullifiers,
            note_commitments,
            encrypted_notes,
            anchor,
            current_total_balance,
            creation_time_ms,
        );

        Ok(result.map(|action| action.into()))
    }
}
