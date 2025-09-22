use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEventInfo;
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
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::identity::PartialIdentity;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::version::PlatformVersion;
use drive::drive::identity::update::apply_balance_change_outcome::ApplyBalanceChangeOutcomeV0Methods;
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[allow(clippy::too_many_arguments)]
    fn paid_function(
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
        event: ExecutionEventInfo,
        consensus_errors: Vec<ConsensusError>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<EventExecutionResult, Error> {
        let maybe_fee_validation_result = match event {
            ExecutionEventInfo::PaidFromAssetLock { .. } | ExecutionEventInfo::Paid { .. } => {
                Some(self.validate_fees_of_event(
                    &event,
                    block_info,
                    Some(transaction),
                    platform_version,
                    previous_fee_versions,
                )?)
            }
            ExecutionEventInfo::PaidFromAssetLockWithoutIdentity { .. }
            | ExecutionEventInfo::PaidFixedCost { .. }
            | ExecutionEventInfo::Free { .. } => None,
        };

        match event {
            ExecutionEventInfo::PaidFromAssetLock {
                identity,
                operations,
                execution_operations,
                user_fee_increase,
                ..
            } => {
                // We can unwrap here because we have the match right above
                let fee_validation_result = maybe_fee_validation_result.unwrap();
                self.paid_function(
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
            ExecutionEventInfo::Paid {
                identity,
                operations,
                execution_operations,
                additional_fixed_fee_cost,
                user_fee_increase,
                ..
            } => {
                // We can unwrap here because we have the match right above
                let fee_validation_result = maybe_fee_validation_result.unwrap();
                self.paid_function(
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
            // This is for Partially used Asset Locks
            // NOT used for identity create or identity top up
            ExecutionEventInfo::PaidFromAssetLockWithoutIdentity {
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
            ExecutionEventInfo::PaidFixedCost {
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
            ExecutionEventInfo::Free { operations } => {
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
