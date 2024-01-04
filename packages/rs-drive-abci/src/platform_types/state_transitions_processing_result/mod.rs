use dpp::consensus::ConsensusError;

use crate::error::Error;
use crate::platform_types::event_execution_result::EstimatedFeeResult;
use dpp::fee::fee_result::FeeResult;

#[derive(Debug, Clone)]
pub enum StateTransitionExecutionResult {
    // TODO: Errors should also have fees
    PaidConsensusError(ConsensusError),
    UnpaidConsensusError(ConsensusError),
    DriveAbciError(String),
    SuccessfulExecution(EstimatedFeeResult, FeeResult),
}

/// An execution result
#[derive(Debug, Default, Clone)]
pub struct StateTransitionsProcessingResult {
    execution_results: Vec<StateTransitionExecutionResult>,
    invalid_count: usize,
    valid_count: usize,
    failed_count: usize,
    fees: FeeResult,
}

impl StateTransitionsProcessingResult {
    pub fn add(&mut self, execution_result: StateTransitionExecutionResult) -> Result<(), Error> {
        match &execution_result {
            StateTransitionExecutionResult::DriveAbciError(_) => {
                self.failed_count += 1;
            }
            StateTransitionExecutionResult::PaidConsensusError(_)
            | StateTransitionExecutionResult::UnpaidConsensusError(_) => {
                self.invalid_count += 1;
            }
            StateTransitionExecutionResult::SuccessfulExecution(_, actual_fees) => {
                self.valid_count += 1;

                self.fees.checked_add_assign(actual_fees.clone())?;
            }
        }

        self.execution_results.push(execution_result);

        Ok(())
    }

    pub fn invalid_count(&self) -> usize {
        self.invalid_count
    }

    pub fn valid_count(&self) -> usize {
        self.valid_count
    }

    pub fn failed_count(&self) -> usize {
        self.failed_count
    }

    pub fn aggregated_fees(&self) -> &FeeResult {
        &self.fees
    }

    pub fn into_execution_results(self) -> Vec<StateTransitionExecutionResult> {
        self.execution_results
    }
}
