use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, validate_anchor_exists, validate_nullifiers,
};
use dpp::block::block_info::BlockInfo;
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
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldedTransferStateTransitionTransformIntoActionValidationV0 for ShieldedTransferTransition {
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Extract nullifiers, note commitments, and encrypted notes from serialized actions
        let nullifiers: Vec<[u8; 32]> = match self {
            ShieldedTransferTransition::V0(v0) => v0.actions.iter().map(|a| a.nullifier).collect(),
        };
        let note_commitments: Vec<[u8; 32]> = match self {
            ShieldedTransferTransition::V0(v0) => v0.actions.iter().map(|a| a.cmx).collect(),
        };
        let encrypted_notes: Vec<Vec<u8>> = match self {
            ShieldedTransferTransition::V0(v0) => v0
                .actions
                .iter()
                .map(|a| a.encrypted_note.clone())
                .collect(),
        };

        // The anchor from the transition (Merkle root of commitment tree)
        let anchor: [u8; 32] = match self {
            ShieldedTransferTransition::V0(v0) => v0.anchor,
        };

        // The value_balance is the fee amount extracted from the shielded pool
        let fee_amount: Credits = match self {
            ShieldedTransferTransition::V0(v0) => v0.value_balance,
        };

        // Read current shielded pool state from GroveDB
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

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

        let result = ShieldedTransferTransitionAction::try_from_transition(
            self,
            nullifiers,
            note_commitments,
            encrypted_notes,
            anchor,
            fee_amount,
            current_total_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}
