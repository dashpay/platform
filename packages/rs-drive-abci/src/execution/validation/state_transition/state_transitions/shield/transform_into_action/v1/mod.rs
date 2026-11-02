use super::v0::reallocate_inputs_for_shield_amount;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, validate_nullifiers,
};
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shield::ShieldTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

pub(in crate::execution::validation::state_transition::state_transitions::shield) trait ShieldStateTransitionTransformIntoActionValidationV1
{
    fn transform_into_action_v1(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldStateTransitionTransformIntoActionValidationV1 for ShieldTransition {
    /// Version 0, plus the nullifier checks every spend runs: each action of an outputs-only
    /// bundle still reveals a nullifier (that of a dummy spend, which becomes the new note's
    /// `rho`), and the action's operations record it, so a nullifier repeated inside the bundle
    /// or already recorded by an earlier spend or shield is refused with
    /// `NullifierAlreadySpentError`.
    ///
    /// The refusal is unpaid, like the spends' and like a failed proof of this transition:
    /// the proof was verified by the stateless step before this transform, and no action is
    /// returned, so proposers leave the transition out of the block and CheckTx refuses it.
    fn transform_into_action_v1(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        _block_info: &BlockInfo,
        _execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let ShieldTransition::V0(transition_v0) = self;
        let shield_amount: Credits = transition_v0.amount;

        // The shared address-balance validation debits the FULL per-input `requested`
        // (a max contribution), but the shielded pool only receives `shield_amount`.
        // Reallocate so the total debited across inputs equals exactly `shield_amount`,
        // leaving the excess in the source addresses. This keeps credits conserved
        // (addresses lose `shield_amount` + fee, pool gains `shield_amount`).
        let inputs_with_remaining_balance = reallocate_inputs_for_shield_amount(
            transition_v0,
            inputs_with_remaining_balance,
            shield_amount,
        )?;

        // Read current shielded pool state from GroveDB.
        //
        // Shield is metered + compute: GroveDB meters the real storage and processing of the
        // note/nullifier writes, and the execution-event layer adds the shielded COMPUTE fee
        // `compute_shielded_verification_fee(num_actions)` (proof verification + per-action
        // processing, which prices the per-action nullifier check) on top as
        // `additional_fixed_fee_cost`. These validation reads are not charged separately, as in
        // the spends.
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

        // Validate nullifiers: intra-bundle duplicates + already recorded in state
        let nullifiers: Vec<[u8; 32]> = transition_v0
            .actions
            .iter()
            .map(|action| action.nullifier)
            .collect();
        if let Some(consensus_error) = validate_nullifiers(
            drive,
            &nullifiers,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        let result = ShieldTransitionAction::try_from_transition(
            self,
            inputs_with_remaining_balance,
            shield_amount,
            current_total_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}
