use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use drive::fee::result::FeeResult;
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
    pub(in crate::execution) fn validate_fees_of_event_v0(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<FeeResult>, Error> {
        match event {
            ExecutionEvent::PaidFromAssetLockDriveEvent {
                identity,
                added_balance,
                operations,
            } => {
                let previous_balance = identity.balance.ok_or(Error::Execution(
                    ExecutionError::CorruptedCodeExecution("partial identity info with no balance"),
                ))?;
                let previous_balance_with_top_up = previous_balance + added_balance;
                let estimated_fee_result = self
                    .drive
                    .apply_drive_operations(operations.clone(), false, block_info, transaction)
                    .map_err(Error::Drive)?;

                // TODO: Should take into account refunds as well
                if previous_balance_with_top_up >= estimated_fee_result.total_base_fee() {
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
                            ),
                        )
                        .into()],
                    ))
                }
            }
            ExecutionEvent::PaidDriveEvent {
                identity,
                operations,
            } => {
                let balance = identity.balance.ok_or(Error::Execution(
                    ExecutionError::CorruptedCodeExecution("partial identity info with no balance"),
                ))?;
                let estimated_fee_result = self
                    .drive
                    .apply_drive_operations(operations.clone(), false, block_info, transaction)
                    .map_err(Error::Drive)?;

                // TODO: Should take into account refunds as well
                if balance >= estimated_fee_result.total_base_fee() {
                    Ok(ConsensusValidationResult::new_with_data(
                        estimated_fee_result,
                    ))
                } else {
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![StateError::IdentityInsufficientBalanceError(
                            IdentityInsufficientBalanceError::new(identity.id, balance),
                        )
                        .into()],
                    ))
                }
            }
            ExecutionEvent::FreeDriveEvent { .. } => Ok(ConsensusValidationResult::new_with_data(
                FeeResult::default(),
            )),
        }
    }
}
