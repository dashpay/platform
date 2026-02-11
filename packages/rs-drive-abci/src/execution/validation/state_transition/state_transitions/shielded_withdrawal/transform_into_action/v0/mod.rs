use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::reconstruct_and_verify_bundle;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::shielded::invalid_anchor_error::InvalidAnchorError;
use dpp::consensus::state::shielded::nullifier_already_spent_error::NullifierAlreadySpentError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_anchors_credit_pool_path, shielded_credit_pool_nullifiers_path,
    shielded_credit_pool_path, SHIELDED_TOTAL_BALANCE_KEY,
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shielded_withdrawal::ShieldedWithdrawalTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use drive::util::grove_operations::DirectQueryType;

pub(in crate::execution::validation::state_transition::state_transitions::shielded_withdrawal) trait ShieldedWithdrawalStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        block_info: &BlockInfo,
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
        let pool_path = shielded_credit_pool_path();

        let current_total_balance = drive
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&pool_path).into(),
                &[SHIELDED_TOTAL_BALANCE_KEY],
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .unwrap_or(0);

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
        let anchors_path = shielded_anchors_credit_pool_path();
        let anchor_exists = drive.grove_has_raw(
            (&anchors_path).into(),
            &anchor,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        if !anchor_exists {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidAnchorError(InvalidAnchorError::new(anchor)).into(),
            ));
        }

        // Check that no nullifier has already been spent
        let nullifiers_path = shielded_credit_pool_nullifiers_path();
        for nullifier in &nullifiers {
            let exists = drive.grove_has_raw(
                (&nullifiers_path).into(),
                nullifier,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;

            if exists {
                return Ok(ConsensusValidationResult::new_with_error(
                    StateError::NullifierAlreadySpentError(NullifierAlreadySpentError::new(
                        *nullifier,
                    ))
                    .into(),
                ));
            }
        }

        // Verify the Orchard ZK proof, binding transparent fields to the sighash.
        // For shielded withdrawal, the extra_sighash_data binds the withdrawal
        // destination (output_script) and amount, preventing an attacker from
        // substituting a different output_script or amount while reusing a valid
        // Orchard bundle.
        let (st_actions, st_flags, st_value_balance, st_proof, st_binding_sig, output_script, amount) =
            match self {
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
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(e).into(),
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
