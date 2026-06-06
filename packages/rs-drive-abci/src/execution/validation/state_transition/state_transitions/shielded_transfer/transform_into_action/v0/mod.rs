use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, validate_anchor_exists, validate_nullifiers,
};
use dpp::consensus::state::state_error::StateError;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shielded_transfer::ShieldedTransferTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::shielded_transfer) trait ShieldedTransferStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldedTransferStateTransitionTransformIntoActionValidationV0 for ShieldedTransferTransition {
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // The value_balance is the fee amount extracted from the shielded pool
        let fee_amount: Credits = match self {
            ShieldedTransferTransition::V0(v0) => v0.value_balance,
        };

        // The anchor from the transition (Merkle root of commitment tree)
        let anchor: [u8; 32] = match self {
            ShieldedTransferTransition::V0(v0) => v0.anchor,
        };

        // Extract nullifiers from the transition actions
        let nullifiers: Vec<[u8; 32]> = match self {
            ShieldedTransferTransition::V0(v0) => v0.actions.iter().map(|a| a.nullifier).collect(),
        };

        // Read current shielded pool state from GroveDB
        //
        // SAFETY: No TOCTOU risk. Shielded transitions are processed sequentially
        // against the same GroveDB transaction. Each transition's operations are
        // applied (via apply_drive_operations) before the next transition's
        // validation runs. GroveDB supports read-your-own-writes within that
        // uncommitted transaction, so balance reads always see the latest state
        // from prior transitions in the block.
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

        // Note: validate_minimum_pool_notes is intentionally NOT called here.
        // Unlike Unshield and ShieldedWithdrawal which reveal a transparent destination
        // (output address or L1 withdrawal), shielded transfers remain entirely within the
        // pool with no visible destination. The minimum pool notes threshold exists to
        // protect the anonymity set when outflows have observable destinations -- it does
        // not apply to pool-internal transfers.

        // Verify the pool has sufficient balance for the fee
        if current_total_balance < fee_amount {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(
                    dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError::new(
                        format!(
                            "shielded pool has insufficient balance: pool has {} but fee requires {}",
                            current_total_balance, fee_amount
                        ),
                    ),
                )
                .into(),
            ));
        }

        // Verify the anchor exists in the recorded anchors tree
        if let Some(consensus_error) = validate_anchor_exists(
            drive,
            &anchor,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        // Validate nullifiers: intra-bundle duplicates + already-spent in state
        if let Some(consensus_error) = validate_nullifiers(
            drive,
            &nullifiers,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        // Shielded transitions do NOT meter the GroveDB operation cost as a fee. They
        // pay a flat, client-predictable fee (`compute_minimum_shielded_fee`) baked into
        // the ZK-proven `value_balance`: the client must know the exact fee offline to
        // build its proof and cannot run `Drive::calculate_fee` (which needs server-side
        // state). The flat fee subsumes these validation reads, so the cost accumulated
        // in `drive_operations` is intentionally not charged — `PaidFromShieldedPool`
        // carves the fee straight from the pool and never consumes the execution context.
        let result = ShieldedTransferTransitionAction::try_from_transition(
            self,
            fee_amount,
            current_total_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}
