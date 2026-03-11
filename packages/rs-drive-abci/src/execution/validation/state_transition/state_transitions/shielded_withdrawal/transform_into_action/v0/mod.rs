use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
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
        let unshielding_amount = match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.unshielding_amount,
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

        // Build the action, which includes creating the withdrawal document
        let creation_time_ms = block_info.time_ms;

        let result = ShieldedWithdrawalTransitionAction::try_from_transition(
            self,
            current_total_balance,
            creation_time_ms,
        );

        Ok(result.map(|action| action.into()))
    }
}
