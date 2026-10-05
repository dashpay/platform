use super::v0::reallocate_inputs_for_shield_amount;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, reconstruct_and_verify_bundle, validate_nullifiers, FLAGS_OUTPUTS_ONLY,
};
use crate::execution::validation::state_transition::ValidationMode;
use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::consensus::basic::state_transition::StateTransitionNotActiveError;
use dpp::consensus::state::address_funds::AddressesNotEnoughFundsError;
use dpp::consensus::state::state_error::StateError;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::shielded::shield_extra_sighash_data;
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use drive::state_transition_action::address_funds::restore_input_spends_for_failed_transition;
use drive::state_transition_action::shielded::shield::ShieldTransitionAction;
use drive::state_transition_action::system::bump_address_input_nonces_action::{
    BumpAddressInputNoncesAction, BumpAddressInputNoncesActionV0,
};
use drive::state_transition_action::StateTransitionAction;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::execution::validation::state_transition::state_transitions::shield) trait ShieldStateTransitionTransformIntoActionValidationV2
{
    #[allow(clippy::too_many_arguments)]
    fn transform_into_action_v2(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        epoch: &Epoch,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldStateTransitionTransformIntoActionValidationV2 for ShieldTransition {
    /// Version 1's nullifier checks and successful shielding, with proof verification
    /// performed here during block validation so an authenticated bad proof can return
    /// a paid nonce-bump action. The input principal is restored, and the fixed penalty
    /// is capped by signed fee-payer funds left after reserving the estimated base fee.
    /// Insufficient base-fee funds and duplicate nullifiers still produce unpaid
    /// refusals; nullifiers are checked before the proof. CheckTx keeps proof
    /// verification in its caller, and rechecks do not verify it again.
    #[allow(clippy::too_many_arguments)]
    fn transform_into_action_v2(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        epoch: &Epoch,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let ShieldTransition::V1(transition) = self else {
            let legacy = StateTransition::Shield(self.clone());
            return Ok(ConsensusValidationResult::new_with_error(
                StateTransitionNotActiveError::new(
                    legacy.name(),
                    platform_version.protocol_version,
                    *legacy.active_version_range().end(),
                )
                .into(),
            ));
        };
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;
        let nullifiers: Vec<_> = transition
            .actions
            .iter()
            .map(|action| action.nullifier)
            .collect();
        if let Some(refusal) = validate_nullifiers(
            drive,
            &nullifiers,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(refusal);
        }

        // CheckTx's caller verifies once under its local proof permit. Recheck and
        // action-only construction do no proof work and cannot prepare paid failures.
        if validation_mode == ValidationMode::Validator {
            let extra_sighash_data =
                shield_extra_sighash_data(&transition.inputs, platform_version)?;
            if let Err(error) = reconstruct_and_verify_bundle(
                &transition.actions,
                FLAGS_OUTPUTS_ONLY,
                -(transition.amount as i64),
                &transition.anchor,
                &transition.proof,
                &transition.binding_signature,
                &extra_sighash_data,
            ) {
                // These balances still reflect the full requested debits. Restore them
                // before successful-path reallocation so a failure moves no principal.
                let mut restored = inputs_with_remaining_balance;
                restore_input_spends_for_failed_transition(&mut restored, &transition.inputs)?;
                let mut bump = BumpAddressInputNoncesActionV0 {
                    inputs_with_remaining_balance: restored,
                    fee_strategy: transition.fee_strategy.clone(),
                    user_fee_increase: transition.user_fee_increase,
                    penalty_credits: 0,
                };
                let operations = BumpAddressInputNoncesAction::from(bump.clone())
                    .into_high_level_drive_operations(epoch, platform_version)?;
                let mut estimated = drive.apply_drive_operations(
                    operations,
                    false,
                    block_info,
                    transaction,
                    platform_version,
                    Some(previous_fee_versions),
                )?;
                ValidationOperation::add_many_to_fee_result(
                    execution_context.operations_slice(),
                    &mut estimated,
                    platform_version,
                )?;
                estimated.apply_user_fee_increase(transition.user_fee_increase);

                let addresses: Vec<_> = bump.inputs_with_remaining_balance.values().collect();
                let mut paying_indices = BTreeSet::new();
                let mut available: Credits = 0;
                for step in &transition.fee_strategy {
                    if let AddressFundsFeeStrategyStep::DeductFromInput(index) = step {
                        if paying_indices.insert(*index) {
                            if let Some((_, balance)) = addresses.get(*index as usize) {
                                available = available.checked_add(*balance).ok_or(
                                    Error::Execution(ExecutionError::Overflow(
                                        "shield failure fee-payer balance overflow",
                                    )),
                                )?;
                            }
                        }
                    }
                }
                let penalty = platform_version
                    .drive_abci
                    .validation_and_processing
                    .penalties
                    .shielded_proof_verification_failure;
                if available < estimated.total_base_fee() {
                    return Ok(ConsensusValidationResult::new_with_error(
                        AddressesNotEnoughFundsError::new(
                            bump.inputs_with_remaining_balance,
                            estimated.total_base_fee(),
                        )
                        .into(),
                    ));
                }
                bump.penalty_credits =
                    penalty.min(available.saturating_sub(estimated.total_base_fee()));

                // Refuse before creating a paid-invalid event when its estimated fee
                // cannot fit. Execution charges the fixed penalty once, without the
                // user's fee increase.
                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    StateTransitionAction::BumpAddressInputNoncesAction(bump.into()),
                    vec![StateError::InvalidShieldedProofError(error).into()],
                ));
            }
        }

        let remaining = reallocate_inputs_for_shield_amount(
            &transition.inputs,
            &transition.fee_strategy,
            inputs_with_remaining_balance,
            transition.amount,
        )?;
        Ok(ShieldTransitionAction::try_from_transition(
            self,
            remaining,
            transition.amount,
            current_total_balance,
        )
        .map(|action| action.into()))
    }
}
