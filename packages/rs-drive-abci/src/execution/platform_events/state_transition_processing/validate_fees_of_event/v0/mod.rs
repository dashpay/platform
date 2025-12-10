use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::address_funds::fee_strategy::deduct_fee_from_inputs_and_outputs::deduct_fee_from_outputs_or_remaining_balance_of_inputs;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::address_funds::AddressesNotEnoughFundsError;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::consensus::state::state_error::StateError;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;

use dpp::prelude::ConsensusValidationResult;
use dpp::version::PlatformVersion;

use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Validates the fees of a given `ExecutionEvent`.
    ///
    /// # Arguments
    ///
    /// * `event` - The `ExecutionEvent` instance to validate.
    /// * `block_info` - Information about the current block.
    /// * `transaction` - The transaction arguments for the given event.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<FeeResult>, Error>` - On success, returns a
    ///   `ConsensusValidationResult` containing an `FeeResult`. On error, returns an `Error`.
    ///
    /// # Errors
    ///
    /// * This function may return an `Error::Execution` if the identity balance is not found.
    /// * This function may return an `Error::Drive` if there's an issue with applying drive operations.
    pub(super) fn validate_fees_of_event_v0(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<ConsensusValidationResult<FeeResult>, Error> {
        match event {
            ExecutionEvent::PaidFromAssetLock {
                identity,
                added_balance,
                operations,
                execution_operations,
                user_fee_increase,
            } => {
                let previous_balance = identity.balance.ok_or(Error::Execution(
                    ExecutionError::CorruptedCodeExecution("partial identity info with no balance in paid from asset lock execution event"),
                ))?;
                let previous_balance_with_top_up = previous_balance.saturating_add(*added_balance);
                let mut estimated_fee_result = self
                    .drive
                    .apply_drive_operations(
                        operations.clone(),
                        false,
                        block_info,
                        transaction,
                        platform_version,
                        Some(previous_fee_versions),
                    )
                    .map_err(Error::Drive)?;

                ValidationOperation::add_many_to_fee_result(
                    execution_operations,
                    &mut estimated_fee_result,
                    platform_version,
                )?;

                estimated_fee_result.apply_user_fee_increase(*user_fee_increase);

                // TODO: Should take into account refunds as well
                let total_fee = estimated_fee_result.total_base_fee();
                if previous_balance_with_top_up >= total_fee {
                    Ok(ConsensusValidationResult::new_with_data(
                        estimated_fee_result,
                    ))
                } else {
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![StateError::IdentityInsufficientBalanceError(
                            IdentityInsufficientBalanceError::new(
                                identity.id,
                                previous_balance_with_top_up,
                                total_fee,
                            ),
                        )
                        .into()],
                    ))
                }
            }
            ExecutionEvent::Paid {
                identity,
                removed_balance,
                operations,
                execution_operations,
                additional_fixed_fee_cost: additional_fee_cost,
                user_fee_increase,
            } => {
                let balance = identity.balance.ok_or(Error::Execution(
                    ExecutionError::CorruptedCodeExecution(
                        "partial identity info with no balance in paid execution event",
                    ),
                ))?;
                let balance_after_principal_operation =
                    balance.saturating_sub(removed_balance.unwrap_or_default());
                let mut estimated_fee_result = self
                    .drive
                    .apply_drive_operations(
                        operations.clone(),
                        false,
                        block_info,
                        transaction,
                        platform_version,
                        Some(previous_fee_versions),
                    )
                    .map_err(Error::Drive)?;

                ValidationOperation::add_many_to_fee_result(
                    execution_operations,
                    &mut estimated_fee_result,
                    platform_version,
                )?;

                estimated_fee_result.apply_user_fee_increase(*user_fee_increase);

                // TODO: Should take into account refunds as well
                let mut required_balance = estimated_fee_result.total_base_fee();

                if let Some(additional_fee_cost) = additional_fee_cost {
                    required_balance = required_balance.saturating_add(*additional_fee_cost);
                }

                if balance_after_principal_operation >= required_balance {
                    Ok(ConsensusValidationResult::new_with_data(
                        estimated_fee_result,
                    ))
                } else {
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![StateError::IdentityInsufficientBalanceError(
                            IdentityInsufficientBalanceError::new(
                                identity.id,
                                balance,
                                required_balance,
                            ),
                        )
                        .into()],
                    ))
                }
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
                let mut estimated_fee_result = self
                    .drive
                    .apply_drive_operations(
                        operations.clone(),
                        false,
                        block_info,
                        transaction,
                        platform_version,
                        Some(previous_fee_versions),
                    )
                    .map_err(Error::Drive)?;

                ValidationOperation::add_many_to_fee_result(
                    execution_operations,
                    &mut estimated_fee_result,
                    platform_version,
                )?;

                estimated_fee_result.apply_user_fee_increase(*user_fee_increase);

                let mut required_balance = estimated_fee_result.total_base_fee();

                if let Some(additional_fixed_fee_cost) = additional_fixed_fee_cost {
                    required_balance = required_balance.saturating_add(*additional_fixed_fee_cost);
                }

                let fee_deduction_result = deduct_fee_from_outputs_or_remaining_balance_of_inputs(
                    input_current_balances.clone(),
                    added_to_balance_outputs.clone(),
                    fee_strategy,
                    required_balance,
                    platform_version,
                )?;

                if fee_deduction_result.fee_fully_covered {
                    Ok(ConsensusValidationResult::new_with_data(
                        estimated_fee_result,
                    ))
                } else {
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![StateError::AddressesNotEnoughFundsError(
                            AddressesNotEnoughFundsError::new(
                                input_current_balances.clone(),
                                required_balance,
                            ),
                        )
                        .into()],
                    ))
                }
            }
            ExecutionEvent::PaidFixedCost { .. }
            | ExecutionEvent::Free { .. }
            | ExecutionEvent::PaidFromAssetLockWithoutIdentity { .. } => Ok(
                ConsensusValidationResult::new_with_data(FeeResult::default()),
            ),
        }
    }
}
