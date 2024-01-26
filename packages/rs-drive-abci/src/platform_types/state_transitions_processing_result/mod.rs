use dpp::consensus::ConsensusError;

use crate::error::Error;
use crate::platform_types::event_execution_result::EstimatedFeeResult;
use dpp::fee::fee_result::FeeResult;

/// State Transition Execution Result represents a result of the single state transition execution.
/// There are four possible outcomes of the state transition execution described by this enum
#[derive(Debug, Clone)]
pub enum StateTransitionExecutionResult {
    // TODO: Error should also have fees
    /// State Transition is invalid, but we have a proved identity associated with it,
    /// and we can deduct processing fees calculated until this validation error happened
    PaidConsensusError(ConsensusError),
    /// State Transition is invalid, but we don't have a proved identity associated with it
    /// so we can't deduct balance.
    UnpaidConsensusError(ConsensusError),
    /// State Transition execution failed due to the internal drive-abci error
    DriveAbciError(String),
    /// State Transition was successfully executed
    SuccessfulExecution(EstimatedFeeResult, FeeResult),
}

/// State Transitions Processing Result produced by [process_raw_state_transitions] and represents
/// a result of a batch state transitions execution. It contains [StateTransitionExecutionResult] for
/// each state transition and aggregated fees.
#[derive(Debug, Default, Clone)]
pub struct StateTransitionsProcessingResult {
    execution_results: Vec<StateTransitionExecutionResult>,
    invalid_paid_count: usize,
    invalid_unpaid_count: usize,
    valid_count: usize,
    failed_count: usize,
    fees: FeeResult,
}

impl StateTransitionsProcessingResult {
    /// Add a new execution result
    pub fn add(&mut self, execution_result: StateTransitionExecutionResult) -> Result<(), Error> {
        match &execution_result {
            StateTransitionExecutionResult::DriveAbciError(_) => {
                self.failed_count += 1;
            }
            StateTransitionExecutionResult::PaidConsensusError(_) => {
                self.invalid_paid_count += 1;
            }
            StateTransitionExecutionResult::UnpaidConsensusError(_) => {
                self.invalid_unpaid_count += 1;
            }
            StateTransitionExecutionResult::SuccessfulExecution(_, actual_fees) => {
                self.valid_count += 1;

                self.fees.checked_add_assign(actual_fees.clone())?;
            }
        }

        self.execution_results.push(execution_result);

        Ok(())
    }

    /// Returns the number of paid invalid state transitions
    pub fn invalid_paid_count(&self) -> usize {
        self.invalid_paid_count
    }

    /// Returns the number of unpaid invalid state transitions
    pub fn invalid_unpaid_count(&self) -> usize {
        self.invalid_unpaid_count
    }

    /// Returns the number of valid state transitions
    pub fn valid_count(&self) -> usize {
        self.valid_count
    }

    /// Returns the number of failed state transitions
    pub fn failed_count(&self) -> usize {
        self.failed_count
    }

    /// Returns the aggregated fees
    pub fn aggregated_fees(&self) -> &FeeResult {
        &self.fees
    }

    /// Transform into the state transition execution results
    pub fn into_execution_results(self) -> Vec<StateTransitionExecutionResult> {
        self.execution_results
    }
}
