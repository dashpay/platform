use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::platform_types::event_execution_result::EventExecutionResult;
use crate::platform_types::event_execution_result::EventExecutionResult::{
    SuccessfulFreeExecution, SuccessfulPaidExecution, UnpaidConsensusExecutionError,
    UnsuccessfulPaidExecution,
};
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use drive::drive::identity::update::apply_balance_change_outcome::ApplyBalanceChangeOutcomeV0Methods;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
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
    pub(super) fn execute_event_v0(
        &self,
        event: ExecutionEvent,
        mut consensus_errors: Vec<ConsensusError>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<EventExecutionResult, Error> {
        let maybe_fee_validation_result = match event {
            ExecutionEvent::PaidFromAssetLock { .. } | ExecutionEvent::Paid { .. } => {
                Some(self.validate_fees_of_event(
                    &event,
                    block_info,
                    Some(transaction),
                    platform_version,
                )?)
            }
            ExecutionEvent::PaidFromAssetLockWithoutIdentity { .. }
            | ExecutionEvent::Free { .. } => None,
        };

        match event {
            ExecutionEvent::PaidFromAssetLock {
                identity,
                operations,
                execution_operations,
                user_fee_increase,
                ..
            }
            | ExecutionEvent::Paid {
                identity,
                operations,
                execution_operations,
                user_fee_increase,
                ..
            } => {
                // We can unwrap here because we have the match right above
                let mut fee_validation_result = maybe_fee_validation_result.unwrap();
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
                        )
                        .map_err(Error::Drive)?;

                    ValidationOperation::add_many_to_fee_result(
                        &execution_operations,
                        &mut individual_fee_result,
                        platform_version,
                    )?;

                    individual_fee_result.apply_user_fee_increase(user_fee_increase);

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
            ExecutionEvent::Free { operations } => {
                self.drive
                    .apply_drive_operations(
                        operations,
                        true,
                        block_info,
                        Some(transaction),
                        platform_version,
                    )
                    .map_err(Error::Drive)?;
                Ok(SuccessfulFreeExecution)
            }
        }
    }
}
