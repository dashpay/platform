use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::execution_operation::OperationLike;
use crate::platform_types::platform::Platform;
use crate::platform_types::state_transition_execution_result::StateTransitionExecutionResult;
use crate::platform_types::state_transition_execution_result::StateTransitionExecutionResult::{
    ConsensusExecutionError, SuccessfulFreeExecution, SuccessfulPaidExecution,
};
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::identity::KeyType;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::identity::update::apply_balance_change_outcome::ApplyBalanceChangeOutcomeV0Methods;
use drive::error::Error::Fee;
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
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransitionExecutionResult, Error> {
        //todo: we need to split out errors
        //  between failed execution and internal errors
        let validation_result =
            self.validate_fees_of_event(&event, block_info, Some(transaction), platform_version)?;

        match event {
            ExecutionEvent::PaidFromAssetLockDriveEvent {
                identity,
                operations,
                signature_verifications,
                ..
            }
            | ExecutionEvent::PaidDriveEvent {
                identity,
                operations,
                signature_verifications,
            } => {
                if validation_result.is_valid_with_data() {
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

                    // Apply signature verification costs
                    if let Some(signature_verifications) = signature_verifications {
                        let total_verification_cost: Credits =
                            signature_verifications.iter().fold(0, |acc, op| {
                                // TODO: handle error
                                let cost = op.processing_cost(platform_version).unwrap();
                                // TODO: handle error?
                                acc.checked_add(cost).unwrap()
                            });
                        // TODO: handle error
                        individual_fee_result
                            .checked_add_assign(FeeResult::default_with_fees(
                                0,
                                total_verification_cost,
                            ))
                            .unwrap();
                    }

                    let balance_change = individual_fee_result.into_balance_change(identity.id);

                    let outcome = self.drive.apply_balance_change_from_fee_to_identity(
                        balance_change,
                        Some(transaction),
                        platform_version,
                    )?;

                    Ok(SuccessfulPaidExecution(
                        validation_result.into_data()?,
                        outcome.actual_fee_paid_owned(),
                    ))
                } else {
                    Ok(ConsensusExecutionError(
                        SimpleConsensusValidationResult::new_with_errors(validation_result.errors),
                    ))
                }
            }
            ExecutionEvent::FreeDriveEvent { operations } => {
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
