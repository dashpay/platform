use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, validate_anchor_exists, validate_minimum_pool_notes,
    validate_nullifiers,
};
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shielded_withdrawal::ShieldedWithdrawalTransitionAction;
use drive::state_transition_action::StateTransitionAction;

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
        // The anchor from the transition (Merkle root of commitment tree)
        let anchor: [u8; 32] = match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.anchor,
        };

        // Extract nullifiers from the transition actions
        let nullifiers: Vec<[u8; 32]> = match self {
            ShieldedWithdrawalTransition::V0(v0) => {
                v0.actions.iter().map(|a| a.nullifier).collect()
            }
        };

        // Read current shielded pool total balance from GroveDB
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

        // Check minimum notes threshold for outgoing transitions (anonymity set)
        if let Some(consensus_error) = validate_minimum_pool_notes(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        // Verify the pool has sufficient balance for the withdrawal.
        let (unshielding_amount, num_actions) = match self {
            ShieldedWithdrawalTransition::V0(v0) => (v0.unshielding_amount, v0.actions.len()),
        };

        if current_total_balance < unshielding_amount {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(
                    dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError::new(
                        format!(
                            "shielded pool has insufficient balance: pool has {} but withdrawal requires {}",
                            current_total_balance, unshielding_amount
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
        // pay a flat, client-predictable fee (`compute_shielded_withdrawal_fee`, computed
        // below): the client must know the exact fee offline to build its proof and
        // cannot run `Drive::calculate_fee` (which needs server-side state). The flat fee
        // subsumes these validation reads, so the cost accumulated in `drive_operations`
        // is intentionally not charged — `PaidFromShieldedPool` carves the fee straight
        // from the pool and never consumes the execution context.

        // The fee charged to the shielded pool is the ShieldedWithdrawal fee computed from the
        // same `num_actions` that `validate_minimum_shielded_fee` enforced the net range against.
        // Unlike the base `compute_minimum_shielded_fee`, this fee ALSO includes the flat storage
        // cost of the Core withdrawal document this transition inserts (`AddWithdrawalDocument` —
        // a real document write into the withdrawals contract plus its index entries), so the
        // booking covers that write instead of diverting the proposer's processing reward to it.
        // Because the validation gate passed using this same `compute_shielded_withdrawal_fee`,
        // the net amount withdrawn to Core (`unshielding_amount - fee_amount`) is guaranteed to
        // fall within `[MIN_WITHDRAWAL_AMOUNT, max_withdrawal_amount]`; the action transformer
        // re-checks that same range with `checked_sub` as defense in depth, so those rejection
        // paths are unreachable for validated input.
        let fee_amount =
            dpp::shielded::compute_shielded_withdrawal_fee(num_actions, platform_version)?;

        // Build the action, which includes creating the withdrawal document
        let creation_time_ms = block_info.time_ms;

        let result = ShieldedWithdrawalTransitionAction::try_from_transition(
            self,
            current_total_balance,
            creation_time_ms,
            fee_amount,
            platform_version,
        );

        Ok(result.map(|action| action.into()))
    }
}
