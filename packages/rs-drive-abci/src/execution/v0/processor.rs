use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::execution_event::ExecutionResult::{
    ConsensusExecutionError, SuccessfulFreeExecution, SuccessfulPaidExecution,
};
use crate::execution::execution_event::{ExecutionEvent, ExecutionResult};
use crate::platform::state::PlatformState;
use crate::platform::{Platform, PlatformRef};
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::processor::process_state_transition;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use drive::fee::result::FeeResult;
use drive::grovedb::{Transaction, TransactionArg};
use tenderdash_abci::proto::abci::ExecTxResult;

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
    pub(crate) fn validate_fees_of_event(
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
    pub(crate) fn execute_event(
        &self,
        event: ExecutionEvent,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<ExecutionResult, Error> {
        //todo: we need to split out errors
        //  between failed execution and internal errors
        let validation_result =
            self.validate_fees_of_event(&event, block_info, Some(transaction))?;
        match event {
            ExecutionEvent::PaidFromAssetLockDriveEvent {
                identity,
                operations,
                ..
            }
            | ExecutionEvent::PaidDriveEvent {
                identity,
                operations,
            } => {
                if validation_result.is_valid_with_data() {
                    //todo: make this into an atomic event with partial batches
                    let individual_fee_result = self
                        .drive
                        .apply_drive_operations(operations, true, block_info, Some(transaction))
                        .map_err(Error::Drive)?;

                    let balance_change =
                        individual_fee_result.into_balance_change(identity.id.to_buffer());

                    let outcome = self.drive.apply_balance_change_from_fee_to_identity(
                        balance_change,
                        Some(transaction),
                    )?;

                    // println!("State transition fees {:#?}", outcome.actual_fee_paid);
                    //
                    // println!(
                    //     "Identity balance {:?} changed {:#?}",
                    //     identity.balance,
                    //     balance_change.change()
                    // );

                    Ok(SuccessfulPaidExecution(
                        validation_result.into_data()?,
                        outcome.actual_fee_paid,
                    ))
                } else {
                    Ok(ConsensusExecutionError(
                        SimpleConsensusValidationResult::new_with_errors(validation_result.errors),
                    ))
                }
            }
            ExecutionEvent::FreeDriveEvent { operations } => {
                self.drive
                    .apply_drive_operations(operations, true, block_info, Some(transaction))
                    .map_err(Error::Drive)?;
                Ok(SuccessfulFreeExecution)
            }
        }
    }

    /// Processes the given raw state transitions based on the `block_info` and `transaction`.
    ///
    /// This function takes a reference to a vector of raw state transitions, `BlockInfo`, and a `Transaction`
    /// as input and performs the corresponding state transition operations. It deserializes the raw state
    /// transitions into a `StateTransition` and processes them.
    ///
    /// # Arguments
    ///
    /// * `raw_state_transitions` - A reference to a vector of raw state transitions.
    /// * `block_info` - Information about the current block being processed.
    /// * `transaction` - The transaction associated with the raw state transitions.
    ///
    /// # Returns
    ///
    /// * `Result<(FeeResult, Vec<ExecTxResult>), Error>` - If the processing is successful, it returns
    ///   a tuple consisting of a `FeeResult` and a vector of `ExecTxResult`. If the processing fails,
    ///   it returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with deserializing the raw
    /// state transitions, processing state transitions, or executing events.
    ///
    pub(crate) fn process_raw_state_transitions(
        &self,
        raw_state_transitions: &Vec<Vec<u8>>,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<(FeeResult, Vec<(Vec<u8>, ExecTxResult)>), Error> {
        let state_transitions = StateTransition::deserialize_many(raw_state_transitions)?;
        let mut aggregate_fee_result = FeeResult::default();
        let platform_ref = PlatformRef {
            drive: &self.drive,
            state: &block_platform_state,
            config: &self.config,
            core_rpc: &self.core_rpc,
        };
        let exec_tx_results = state_transitions
            .into_iter()
            .zip(raw_state_transitions.iter())
            .map(|(state_transition, raw_state_transition)| {
                let state_transition_execution_event =
                    process_state_transition(&platform_ref, state_transition, Some(transaction))?;

                let execution_result = if state_transition_execution_event.is_valid() {
                    let execution_event = state_transition_execution_event.into_data()?;
                    self.execute_event(execution_event, block_info, transaction)?
                } else {
                    ConsensusExecutionError(SimpleConsensusValidationResult::new_with_errors(
                        state_transition_execution_event.errors,
                    ))
                };
                if let SuccessfulPaidExecution(_, fee_result) = &execution_result {
                    aggregate_fee_result.checked_add_assign(fee_result.clone())?;
                }

                Ok((raw_state_transition.clone(), execution_result.into()))
            })
            .collect::<Result<Vec<(Vec<u8>, ExecTxResult)>, Error>>()?;
        Ok((aggregate_fee_result, exec_tx_results))
    }
}
