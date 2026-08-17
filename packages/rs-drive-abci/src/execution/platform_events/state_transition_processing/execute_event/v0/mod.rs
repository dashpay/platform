use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::platform_events::state_transition_processing::record_added_balance_outputs::AddedBalanceOutputsOrigin;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::platform_types::event_execution_result::EventExecutionResult;
use crate::platform_types::event_execution_result::EventExecutionResult::{
    SuccessfulFreeExecution, SuccessfulPaidExecution, UnpaidConsensusExecutionError,
    UnsuccessfulPaidExecution,
};
use crate::platform_types::platform::Platform;
use std::collections::BTreeMap;

use crate::rpc::core::CoreRPCLike;
use dpp::address_funds::fee_strategy::deduct_fee_from_inputs_and_outputs::deduct_fee_from_outputs_or_remaining_balance_of_inputs;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::balances::credits::CreditOperation;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::identity::PartialIdentity;
use dpp::prelude::{AddressNonce, ConsensusValidationResult, UserFeeIncrease};
use dpp::version::PlatformVersion;
use drive::drive::identity::update::apply_balance_change_outcome::ApplyBalanceChangeOutcomeV0Methods;
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[allow(clippy::too_many_arguments)]
    fn paid_from_identity_function(
        &self,
        mut fee_validation_result: ConsensusValidationResult<FeeResult>,
        identity: PartialIdentity,
        operations: Vec<DriveOperation>,
        execution_operations: Vec<ValidationOperation>,
        user_fee_increase: UserFeeIncrease,
        additional_fixed_fee_cost: Option<Credits>,
        block_info: &BlockInfo,
        mut consensus_errors: Vec<ConsensusError>,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<EventExecutionResult, Error> {
        if fee_validation_result.is_valid_with_data() {
            //todo: make this into an atomic event with partial batches
            let mut individual_fee_result = self
                .drive
                .apply_drive_operations(
                    operations,
                    true,
                    block_info,
                    Some(transaction),
                    platform_version,
                    Some(previous_fee_versions),
                )
                .map_err(Error::Drive)?;

            ValidationOperation::add_many_to_fee_result(
                &execution_operations,
                &mut individual_fee_result,
                platform_version,
            )?;

            individual_fee_result.apply_user_fee_increase(user_fee_increase);

            if let Some(additional_fixed_fee_cost) = additional_fixed_fee_cost {
                individual_fee_result.processing_fee = individual_fee_result
                    .processing_fee
                    .saturating_add(additional_fixed_fee_cost);
            }

            let balance_change = individual_fee_result.into_balance_change(identity.id);

            let outcome = self.drive.apply_balance_change_from_fee_to_identity(
                balance_change,
                Some(transaction),
                platform_version,
            )?;

            if consensus_errors.is_empty() {
                Ok(SuccessfulPaidExecution(
                    Some(fee_validation_result.into_data()?),
                    outcome.actual_fee_paid_owned(),
                ))
            } else {
                Ok(UnsuccessfulPaidExecution(
                    Some(fee_validation_result.into_data()?),
                    outcome.actual_fee_paid_owned(),
                    consensus_errors,
                ))
            }
        } else {
            consensus_errors.append(&mut fee_validation_result.errors);
            Ok(UnpaidConsensusExecutionError(consensus_errors))
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paid_from_address_inputs_and_outputs(
        &self,
        mut fee_validation_result: ConsensusValidationResult<FeeResult>,
        input_current_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        added_to_balance_outputs: BTreeMap<PlatformAddress, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        operations: Vec<DriveOperation>,
        execution_operations: Vec<ValidationOperation>,
        user_fee_increase: UserFeeIncrease,
        additional_fixed_fee_cost: Option<Credits>,
        block_info: &BlockInfo,
        mut consensus_errors: Vec<ConsensusError>,
        transaction: &Transaction,
        mut address_balances_in_update: Option<&mut BTreeMap<PlatformAddress, CreditOperation>>,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<EventExecutionResult, Error> {
        if fee_validation_result.is_valid_with_data() {
            // Apply the drive operations first to calculate the fee
            let mut individual_fee_result = self
                .drive
                .apply_drive_operations(
                    operations,
                    true,
                    block_info,
                    Some(transaction),
                    platform_version,
                    Some(previous_fee_versions),
                )
                .map_err(Error::Drive)?;

            ValidationOperation::add_many_to_fee_result(
                &execution_operations,
                &mut individual_fee_result,
                platform_version,
            )?;

            individual_fee_result.apply_user_fee_increase(user_fee_increase);

            if let Some(additional_fixed_fee_cost) = additional_fixed_fee_cost {
                individual_fee_result.processing_fee = individual_fee_result
                    .processing_fee
                    .saturating_add(additional_fixed_fee_cost);
            }

            // Address-input events have no identity to attribute storage refunds to, so their
            // metered ops must not free identity-attributed storage (`Shield` /
            // `IdentityCreateFromAddresses` only insert or overwrite). A non-empty `fee_refunds`
            // here would have no owner to refund at block fee distribution; assert it in debug/test
            // builds so any future op that frees such storage is caught before it can misbook.
            debug_assert!(
                individual_fee_result.fee_refunds.0.is_empty(),
                "PaidFromAddressInputs ops must not free identity-attributed storage (fee_refunds must be empty)"
            );

            // The total fee to deduct is the metered base fee (storage + processing), where
            // processing already includes any `additional_fixed_fee_cost` added just above. For the
            // transparent `Shield` that fixed cost is the shielded COMPUTE fee
            // (`compute_shielded_verification_fee`), so the fee is `metered_storage + metered_processing +
            // compute_fee` — storage comes entirely from GroveDB metering of the note/nullifier
            // writes, never double-counted. This is identical to every other `PaidFromAddressInputs`
            // event (e.g. `IdentityCreateFromAddresses`), so conservation (deduct == book) holds by
            // the standard machinery: the same `total_fee` is deducted from inputs and booked to the
            // pools. `validate_fees_of_event` rejects an under-funded transition before execution.
            let total_fee = individual_fee_result.total_base_fee();

            // Deduct fee from outputs or remaining balance of inputs according to strategy
            let fee_deduction_result = deduct_fee_from_outputs_or_remaining_balance_of_inputs(
                input_current_balances.clone(),
                added_to_balance_outputs.clone(),
                &fee_strategy,
                total_fee,
                platform_version,
            )?;

            // Defense in depth: the deduction min-caps each step, so an under-funded input set would
            // remove < `total_fee` from the inputs while the full `total_fee` is still booked to the
            // fee pools — minting the difference (`CorruptedCreditsNotBalanced` -> chain halt).
            // `validate_fees_of_event` runs on the same state immediately before execution, but it
            // prices the batch with the ESTIMATED cost model (`apply_drive_operations` with
            // `apply = false`) while `total_fee` here is the ACTUAL metered cost — so this guard
            // triggers whenever `estimated < actual` for a transition funded in between (the
            // mainnet evo1 stalls of 2026-08-14/15; the keyless commitment-tree append is skipped
            // in estimation, dashpay/grovedb#812). The invariant this guard actually enforces is
            // `estimated >= actual`. Note the ops were already applied above: an Err here leaves
            // this transition's writes in the block transaction, which is only safe because the
            // v14+ processing loop rolls dropped transitions back (process_raw_state_transitions
            // v1); under v0 those writes leak into the proposer's app hash.
            if !fee_deduction_result.fee_fully_covered {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "address-input fee not fully covered at execution; validate_fees_of_event should have rejected the under-funded transition",
                )));
            }

            let adjusted_inputs = fee_deduction_result.remaining_input_balances;
            let adjusted_outputs = fee_deduction_result.adjusted_outputs;

            // Now apply the fee adjustments to the state
            // For outputs: compare original with adjusted and remove the difference
            let mut fee_drive_operations = vec![];

            for (address, original_amount) in added_to_balance_outputs.iter() {
                let adjusted_amount = adjusted_outputs.get(address).copied().unwrap_or(0);
                if original_amount > &adjusted_amount {
                    let amount_to_remove = original_amount - adjusted_amount;
                    self.drive.remove_balance_from_address(
                        *address,
                        amount_to_remove,
                        &mut None,
                        &mut fee_drive_operations,
                        Some(transaction),
                        platform_version,
                    )?;

                    // Track the adjusted amount being added (after fee deduction)
                    if let Some(balance_updates) = address_balances_in_update.as_mut() {
                        let new_op = CreditOperation::AddToCredits(adjusted_amount);
                        balance_updates
                            .entry(*address)
                            .and_modify(|existing| {
                                *existing = match existing {
                                    // Set + Add = Set to combined value
                                    CreditOperation::SetCredits(set_val) => {
                                        CreditOperation::SetCredits(
                                            set_val.saturating_add(adjusted_amount),
                                        )
                                    }
                                    // Add + Add = saturating add
                                    CreditOperation::AddToCredits(add_val) => {
                                        CreditOperation::AddToCredits(
                                            add_val.saturating_add(adjusted_amount),
                                        )
                                    }
                                };
                            })
                            .or_insert(new_op);
                    }
                } else {
                    // No fee taken from this output, track the full amount being added
                    if let Some(balance_updates) = address_balances_in_update.as_mut() {
                        let new_op = CreditOperation::AddToCredits(adjusted_amount);
                        balance_updates
                            .entry(*address)
                            .and_modify(|existing| {
                                *existing = match existing {
                                    // Set + Add = Set to combined value
                                    CreditOperation::SetCredits(set_val) => {
                                        CreditOperation::SetCredits(
                                            set_val.saturating_add(adjusted_amount),
                                        )
                                    }
                                    // Add + Add = saturating add
                                    CreditOperation::AddToCredits(add_val) => {
                                        CreditOperation::AddToCredits(
                                            add_val.saturating_add(adjusted_amount),
                                        )
                                    }
                                };
                            })
                            .or_insert(new_op);
                    }
                }
            }

            // For inputs: compare original remaining balance with adjusted and remove the difference
            for (address, (nonce, original_remaining)) in input_current_balances.iter() {
                let adjusted_remaining = adjusted_inputs
                    .get(address)
                    .map(|(_, bal)| *bal)
                    .unwrap_or(0);
                if original_remaining > &adjusted_remaining {
                    // Fees were taken from this input, need to adjust the balance
                    self.drive.set_balance_to_address(
                        *address,
                        *nonce,
                        adjusted_remaining,
                        &mut None,
                        &mut fee_drive_operations,
                        platform_version,
                    )?;
                }

                // Always track the Set operation for input addresses, regardless of fee deduction
                // The input's balance was already set by the drive operations
                if let Some(balance_updates) = address_balances_in_update.as_mut() {
                    let new_op = CreditOperation::SetCredits(adjusted_remaining);
                    balance_updates
                        .entry(*address)
                        .and_modify(|existing| {
                            *existing = match existing {
                                // Set + Set = second Set wins
                                CreditOperation::SetCredits(_) => {
                                    CreditOperation::SetCredits(adjusted_remaining)
                                }
                                // Add + Set = Set (discard the Add)
                                CreditOperation::AddToCredits(_) => {
                                    CreditOperation::SetCredits(adjusted_remaining)
                                }
                            };
                        })
                        .or_insert(new_op);
                }
            }

            // Apply the fee adjustment operations
            if !fee_drive_operations.is_empty() {
                self.drive
                    .apply_batch_low_level_drive_operations(
                        None,
                        Some(transaction),
                        fee_drive_operations,
                        &mut vec![],
                        &platform_version.drive,
                    )
                    .map_err(Error::Drive)?;
            }

            if consensus_errors.is_empty() {
                Ok(SuccessfulPaidExecution(
                    Some(fee_validation_result.into_data()?),
                    individual_fee_result,
                ))
            } else {
                Ok(UnsuccessfulPaidExecution(
                    Some(fee_validation_result.into_data()?),
                    individual_fee_result,
                    consensus_errors,
                ))
            }
        } else {
            consensus_errors.append(&mut fee_validation_result.errors);
            Ok(UnpaidConsensusExecutionError(consensus_errors))
        }
    }

    /// Executes the given `event` based on the `block_info` and `transaction`.
    ///
    /// This function takes an `ExecutionEvent`, `BlockInfo`, and `Transaction` as input and performs
    /// the corresponding operations on the drive. It will validate the fees of the event and apply
    /// drive operations accordingly.
    ///
    /// # Arguments
    ///
    /// * `event` - The execution event to be processed.
    /// * `block_info` - Information about the current block being processed.
    /// * `transaction` - The transaction associated with the execution event.
    ///
    /// # Returns
    ///
    /// * `Result<ExecutionResult, Error>` - If the execution is successful, it returns an `ExecutionResult`
    ///   which can be one of the following variants: `SuccessfulPaidExecution`, `SuccessfulFreeExecution`, or
    ///   `ConsensusExecutionError`. If the execution fails, it returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with the drive operations or
    /// an internal error occurs.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_event_v0(
        &self,
        event: ExecutionEvent,
        consensus_errors: Vec<ConsensusError>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        address_balances_in_update: Option<&mut BTreeMap<PlatformAddress, CreditOperation>>,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<EventExecutionResult, Error> {
        let maybe_fee_validation_result = match event {
            ExecutionEvent::PaidFromAssetLock { .. }
            | ExecutionEvent::Paid { .. }
            | ExecutionEvent::PaidFromAddressInputs { .. }
            | ExecutionEvent::PaidFromAssetLockToPool { .. }
            | ExecutionEvent::PaidFromShieldedPoolToNewIdentity { .. } => {
                Some(self.validate_fees_of_event(
                    &event,
                    block_info,
                    Some(transaction),
                    platform_version,
                    previous_fee_versions,
                )?)
            }
            ExecutionEvent::PaidFromAssetLockWithoutIdentity { .. }
            | ExecutionEvent::PaidFixedCost { .. }
            | ExecutionEvent::PaidFromShieldedPool { .. }
            | ExecutionEvent::Free { .. } => None,
        };

        match event {
            ExecutionEvent::PaidFromAssetLock {
                identity,
                operations,
                execution_operations,
                user_fee_increase,
                ..
            } => {
                // We can unwrap here because we have the match right above
                let fee_validation_result = maybe_fee_validation_result.unwrap();
                self.paid_from_identity_function(
                    fee_validation_result,
                    identity,
                    operations,
                    execution_operations,
                    user_fee_increase,
                    None,
                    block_info,
                    consensus_errors,
                    transaction,
                    platform_version,
                    previous_fee_versions,
                )
            }
            ExecutionEvent::Paid {
                identity,
                added_to_balance_outputs,
                operations,
                execution_operations,
                additional_fixed_fee_cost,
                user_fee_increase,
                ..
            } => {
                // We can unwrap here because we have the match right above
                let fee_validation_result = maybe_fee_validation_result.unwrap();
                let result = self.paid_from_identity_function(
                    fee_validation_result,
                    identity,
                    operations,
                    execution_operations,
                    user_fee_increase,
                    additional_fixed_fee_cost,
                    block_info,
                    consensus_errors,
                    transaction,
                    platform_version,
                    previous_fee_versions,
                )?;

                // Track address outputs if provided (e.g., for IdentityCreditTransferToAddresses).
                // Transparent origin: this long-standing recording is preserved by every version
                // (v0 and v1 both fold Transparent-origin outputs).
                self.record_added_balance_outputs(
                    address_balances_in_update,
                    added_to_balance_outputs,
                    AddedBalanceOutputsOrigin::Transparent,
                    platform_version,
                )?;

                Ok(result)
            }
            ExecutionEvent::PaidFromAddressInputs {
                input_current_balances,
                added_to_balance_outputs,
                fee_strategy,
                operations,
                execution_operations,
                additional_fixed_fee_cost,
                user_fee_increase,
            } => {
                // We can unwrap here because we have the match right above
                let fee_validation_result = maybe_fee_validation_result.unwrap();
                self.paid_from_address_inputs_and_outputs(
                    fee_validation_result,
                    input_current_balances,
                    added_to_balance_outputs,
                    fee_strategy,
                    operations,
                    execution_operations,
                    user_fee_increase,
                    additional_fixed_fee_cost,
                    block_info,
                    consensus_errors,
                    transaction,
                    address_balances_in_update,
                    platform_version,
                    previous_fee_versions,
                )
            }
            // This is for Partially used Asset Locks
            // NOT used for identity create or identity top up
            ExecutionEvent::PaidFromAssetLockWithoutIdentity {
                processing_fees,
                operations,
            } => {
                self.drive
                    .apply_drive_operations(
                        operations,
                        true,
                        block_info,
                        Some(transaction),
                        platform_version,
                        Some(previous_fee_versions),
                    )
                    .map_err(Error::Drive)?;

                if consensus_errors.is_empty() {
                    Ok(SuccessfulPaidExecution(
                        None,
                        FeeResult::default_with_fees(0, processing_fees),
                    ))
                } else {
                    Ok(UnsuccessfulPaidExecution(
                        None,
                        FeeResult::default_with_fees(0, processing_fees),
                        consensus_errors,
                    ))
                }
            }
            ExecutionEvent::PaidFixedCost {
                operations,
                fees_to_add_to_pool,
            } => {
                if consensus_errors.is_empty() {
                    self.drive
                        .apply_drive_operations(
                            operations,
                            true,
                            block_info,
                            Some(transaction),
                            platform_version,
                            Some(previous_fee_versions),
                        )
                        .map_err(Error::Drive)?;

                    Ok(SuccessfulPaidExecution(
                        None,
                        FeeResult::default_with_fees(0, fees_to_add_to_pool),
                    ))
                } else {
                    Ok(UnpaidConsensusExecutionError(consensus_errors))
                }
            }
            ExecutionEvent::PaidFromShieldedPool {
                operations,
                fees_to_add_to_pool,
                added_to_balance_outputs,
                chargeable_failure,
            } => {
                // An error-bearing `PaidFromShieldedPool` is legitimate ONLY for the
                // `IdentityCreateFromShieldedPool` chargeable fallback (`chargeable_failure == true`):
                // the spend must still be finalized (nullifiers consumed, pool debited, fallback
                // address credited) and the penalty booked — exactly like `PaidFromAddressInputs`'
                // `UnsuccessfulPaidExecution` path for the `BumpAddressInputNonces` penalty. If those
                // ops were skipped, the nullifiers would NOT be consumed, letting an attacker force
                // repeated (expensive) Halo 2 verification of the same valid-proof-but-colliding-key
                // transition for free.
                //
                // Every ordinary shielded spend (Unshield / ShieldedTransfer / ShieldedWithdrawal) is
                // data-only on success and error-only on rejection, so it NEVER carries data+errors
                // here and always has `chargeable_failure == false`. If one ever did (a future misuse
                // of `new_with_data_and_errors`), fail SAFE — do NOT commit a side-effectful spend or
                // pay the proposer for a rejected transition. This is consensus-execution code, so we
                // must behave identically in debug and release (a `debug_assert!` would panic in debug
                // but silently fall through in release — a non-deterministic divergence): log the
                // unexpected state at `error` and return the unpaid rejection in BOTH builds.
                if !consensus_errors.is_empty() && !chargeable_failure {
                    tracing::error!(
                        "a non-fallback PaidFromShieldedPool carried consensus errors; rejecting \
                         unpaid (no spend committed, proposer not paid)"
                    );
                    return Ok(UnpaidConsensusExecutionError(consensus_errors));
                }

                let applied_fees = self
                    .drive
                    .apply_drive_operations(
                        operations,
                        true,
                        block_info,
                        Some(transaction),
                        platform_version,
                        Some(previous_fee_versions),
                    )
                    .map_err(Error::Drive)?;

                // The ops just applied credited any transparent output address (an Unshield's
                // recipient, including the chargeable-failure fallback address). Record that credit
                // so incremental client sync sees it. ShieldedSpend origin: recorded only from v1
                // (protocol v13); v0 drops it so pre-v13 blocks byte-match. Reached only on the
                // applied path (the early return above skips this).
                self.record_added_balance_outputs(
                    address_balances_in_update,
                    added_to_balance_outputs,
                    AddedBalanceOutputsOrigin::ShieldedSpend,
                    platform_version,
                )?;

                // Split the carved fee like every other transition: the real storage
                // cost of the (permanent) shielded writes goes to the storage pool, so it
                // is amortised to the validators that store it over time and picks up the
                // epoch fee multiplier at payout; the remainder (proof verification +
                // per-action processing) is the processing fee paid to the current
                // proposer. Conservation: storage + processing == fees_to_add_to_pool
                // (what was carved from the shielded pool).
                let storage_fee = applied_fees.storage_fee.min(fees_to_add_to_pool);
                let processing_fee = fees_to_add_to_pool - storage_fee;
                let fee_result = FeeResult::default_with_fees(storage_fee, processing_fee);

                if consensus_errors.is_empty() {
                    Ok(SuccessfulPaidExecution(None, fee_result))
                } else {
                    // The fallback charged its penalty but the identity was NOT created — report it
                    // as a chargeable consensus failure (the ops above are committed regardless).
                    Ok(UnsuccessfulPaidExecution(
                        None,
                        fee_result,
                        consensus_errors,
                    ))
                }
            }
            ExecutionEvent::PaidFromAssetLockToPool {
                fees_to_add_to_pool,
                added_to_balance_outputs,
                operations,
                ..
            } => {
                let fee_validation_result = maybe_fee_validation_result
                    .expect("fee validation result must exist for PaidFromAssetLockToPool");
                let mut all_errors = fee_validation_result.errors;
                all_errors.extend(consensus_errors);

                if all_errors.is_empty() {
                    let applied_fees = self
                        .drive
                        .apply_drive_operations(
                            operations,
                            true,
                            block_info,
                            Some(transaction),
                            platform_version,
                            Some(previous_fee_versions),
                        )
                        .map_err(Error::Drive)?;

                    // The ops just applied credited the shield's transparent surplus-output address
                    // (when set). Record that credit so incremental client sync sees it. ShieldedSpend
                    // origin: recorded only from v1 (protocol v13); v0 drops it so pre-v13 blocks
                    // byte-match. Reached only on the applied (no-errors) path.
                    self.record_added_balance_outputs(
                        address_balances_in_update,
                        added_to_balance_outputs,
                        AddedBalanceOutputsOrigin::ShieldedSpend,
                        platform_version,
                    )?;

                    // Route the real storage cost of the shielded writes to the storage pool
                    // (amortised over time, epoch fee multiplier applied at payout); the
                    // remainder is the processing fee paid to the proposer. Conservation:
                    // storage + processing == fees_to_add_to_pool.
                    //
                    // `fees_to_add_to_pool` is `pool_fee = compute_minimum_shielded_fee + albc`
                    // (plus any surplus folded in when no surplus_output is set). Since that flat
                    // fee includes the 100M proof-verification fee, it comfortably exceeds the
                    // per-action storage cost, so `processing_fee` is strictly positive — the
                    // proposer is always paid for the proof verification it ran.
                    let storage_fee = applied_fees.storage_fee.min(fees_to_add_to_pool);
                    let processing_fee = fees_to_add_to_pool - storage_fee;

                    Ok(SuccessfulPaidExecution(
                        None,
                        FeeResult::default_with_fees(storage_fee, processing_fee),
                    ))
                } else {
                    Ok(UnpaidConsensusExecutionError(all_errors))
                }
            }
            ExecutionEvent::PaidFromShieldedPoolToNewIdentity {
                identity,
                operations,
                execution_operations,
                additional_fixed_fee_cost,
                ..
            } => {
                // Reuse the create-then-deduct machinery: `paid_from_identity_function` applies the
                // ops (which create the identity holding the full `denomination` and debit the
                // shielded pool — the credits move within the RHS balance trees, so no
                // `AddToSystemCredits`/`RemoveFromSystemCredits` is emitted), then deducts the
                // metered fee + the `additional_fixed_fee_cost` (the shielded compute fee) from the
                // new identity's balance and books it to the fee pools — so the identity ends with
                // `denomination - total_fee`. Conservation holds because the pool-to-identity move
                // stays within the right-hand-side balance trees. Shielded transitions have no fee
                // bidding, so `user_fee_increase` is 0.
                let fee_validation_result = maybe_fee_validation_result.unwrap();
                self.paid_from_identity_function(
                    fee_validation_result,
                    identity,
                    operations,
                    execution_operations,
                    0,
                    additional_fixed_fee_cost,
                    block_info,
                    consensus_errors,
                    transaction,
                    platform_version,
                    previous_fee_versions,
                )
            }
            ExecutionEvent::Free { operations } => {
                self.drive
                    .apply_drive_operations(
                        operations,
                        true,
                        block_info,
                        Some(transaction),
                        platform_version,
                        Some(previous_fee_versions),
                    )
                    .map_err(Error::Drive)?;
                Ok(SuccessfulFreeExecution)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::event_execution_result::EventExecutionResult;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::fee::default_costs::CachedEpochIndexFeeVersions;

    /// Regression for the recent-address-balance-changes gap: a `PaidFromShieldedPool` carrying an
    /// Unshield's output credit must fold that credit into the `address_balances_in_update` map (the
    /// sole feed for `store_address_balances_for_block`), so incremental client sync can see the
    /// unshield. Before the fix the arm applied the drive ops but never touched the map, leaving the
    /// credit invisible to incremental sync.
    #[test]
    fn paid_from_shielded_pool_folds_output_credit_into_address_balances() {
        // Pin v13: the executor's record_added_balance_outputs is v1 here, which folds
        // shielded-spend-origin outputs (v0 would drop them).
        let platform_version = PlatformVersion::get(13).expect("v13 must exist");
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();
        let fee_versions = CachedEpochIndexFeeVersions::new();

        let output_address = PlatformAddress::P2pkh([0xBB; 20]);
        let event = ExecutionEvent::PaidFromShieldedPool {
            operations: vec![],
            fees_to_add_to_pool: 0,
            added_to_balance_outputs: Some(BTreeMap::from([(output_address, 2500)])),
            chargeable_failure: false,
        };

        let mut address_balances: BTreeMap<PlatformAddress, CreditOperation> = BTreeMap::new();
        let result = platform
            .platform
            .execute_event_v0(
                event,
                vec![],
                &BlockInfo::default(),
                &transaction,
                Some(&mut address_balances),
                platform_version,
                &fee_versions,
            )
            .expect("execute_event_v0 should not error");

        assert!(
            matches!(result, EventExecutionResult::SuccessfulPaidExecution(..)),
            "an ordinary unshield must execute successfully"
        );
        assert_eq!(
            address_balances.get(&output_address),
            Some(&CreditOperation::AddToCredits(2500)),
            "the unshield output credit must be recorded for incremental sync"
        );
    }

    /// A `PaidFromShieldedPool` with no output (ShieldedTransfer / ShieldedWithdrawal, or a net-zero
    /// unshield) must leave the address-balances map untouched.
    #[test]
    fn paid_from_shielded_pool_without_output_records_nothing() {
        // Pin v13: the executor's record_added_balance_outputs is v1 here, which folds
        // shielded-spend-origin outputs (v0 would drop them).
        let platform_version = PlatformVersion::get(13).expect("v13 must exist");
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();
        let fee_versions = CachedEpochIndexFeeVersions::new();

        let event = ExecutionEvent::PaidFromShieldedPool {
            operations: vec![],
            fees_to_add_to_pool: 0,
            added_to_balance_outputs: None,
            chargeable_failure: false,
        };

        let mut address_balances: BTreeMap<PlatformAddress, CreditOperation> = BTreeMap::new();
        platform
            .platform
            .execute_event_v0(
                event,
                vec![],
                &BlockInfo::default(),
                &transaction,
                Some(&mut address_balances),
                platform_version,
                &fee_versions,
            )
            .expect("execute_event_v0 should not error");

        assert!(
            address_balances.is_empty(),
            "a shielded spend crediting no transparent address must record nothing"
        );
    }

    /// The `PaidFromAssetLockToPool` counterpart: a `ShieldFromAssetLock` surplus credit must be
    /// folded into the address-balances map on the applied (no-errors) path.
    #[test]
    fn paid_from_asset_lock_to_pool_folds_surplus_credit_into_address_balances() {
        // Pin v13: the executor's record_added_balance_outputs is v1 here, which folds
        // shielded-spend-origin outputs (v0 would drop them).
        let platform_version = PlatformVersion::get(13).expect("v13 must exist");
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();
        let fee_versions = CachedEpochIndexFeeVersions::new();

        let surplus_address = PlatformAddress::P2pkh([0x42; 20]);
        let event = ExecutionEvent::PaidFromAssetLockToPool {
            fees_to_add_to_pool: 1_000_000,
            added_to_balance_outputs: Some(BTreeMap::from([(surplus_address, 2000)])),
            operations: vec![],
            execution_operations: vec![],
        };

        let mut address_balances: BTreeMap<PlatformAddress, CreditOperation> = BTreeMap::new();
        let result = platform
            .platform
            .execute_event_v0(
                event,
                vec![],
                &BlockInfo::default(),
                &transaction,
                Some(&mut address_balances),
                platform_version,
                &fee_versions,
            )
            .expect("execute_event_v0 should not error");

        assert!(
            matches!(result, EventExecutionResult::SuccessfulPaidExecution(..)),
            "a sufficiently-funded shield-from-asset-lock must execute successfully"
        );
        assert_eq!(
            address_balances.get(&surplus_address),
            Some(&CreditOperation::AddToCredits(2000)),
            "the shield surplus credit must be recorded for incremental sync"
        );
    }
}
