use crate::error::Error;
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

            // Get the total fee to deduct
            let total_fee = individual_fee_result.total_base_fee();

            // Deduct fee from outputs or remaining balance of inputs according to strategy
            let (adjusted_inputs, adjusted_outputs) =
                deduct_fee_from_outputs_or_remaining_balance_of_inputs(
                    input_current_balances.clone(),
                    added_to_balance_outputs.clone(),
                    fee_strategy,
                    total_fee,
                    platform_version,
                )?;

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
                }
            }

            // For inputs: compare original remaining balance with adjusted and remove the difference
            for (address, (nonce, original_remaining)) in input_current_balances.iter() {
                let adjusted_remaining = adjusted_inputs
                    .get(address)
                    .map(|(_, bal)| *bal)
                    .unwrap_or(0);
                if original_remaining > &adjusted_remaining {
                    self.drive.set_balance_to_address(
                        *address,
                        *nonce,
                        adjusted_remaining,
                        &mut None,
                        &mut fee_drive_operations,
                        platform_version,
                    )?;
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

            let fee_result = FeeResult::default_with_fees(0, total_fee);

            if consensus_errors.is_empty() {
                Ok(SuccessfulPaidExecution(
                    Some(fee_validation_result.into_data()?),
                    fee_result,
                ))
            } else {
                Ok(UnsuccessfulPaidExecution(
                    Some(fee_validation_result.into_data()?),
                    fee_result,
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
    pub(super) fn execute_event_v0(
        &self,
        event: ExecutionEvent,
        consensus_errors: Vec<ConsensusError>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<EventExecutionResult, Error> {
        let maybe_fee_validation_result = match event {
            ExecutionEvent::PaidFromAssetLock { .. }
            | ExecutionEvent::Paid { .. }
            | ExecutionEvent::PaidFromAddressInputs { .. } => Some(self.validate_fees_of_event(
                &event,
                block_info,
                Some(transaction),
                platform_version,
                previous_fee_versions,
            )?),
            ExecutionEvent::PaidFromAssetLockWithoutIdentity { .. }
            | ExecutionEvent::PaidFixedCost { .. }
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
                operations,
                execution_operations,
                additional_fixed_fee_cost,
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
                    additional_fixed_fee_cost,
                    block_info,
                    consensus_errors,
                    transaction,
                    platform_version,
                    previous_fee_versions,
                )
            }
            ExecutionEvent::PaidFromAddressInputs {
                input_current_balances,
                added_to_balance_outputs,
                fee_strategy,
                operations,
                execution_operations,
                additional_fixed_fee_cost,
                user_fee_increase,
                ..
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
