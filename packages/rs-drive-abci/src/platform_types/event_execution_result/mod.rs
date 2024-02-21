use dpp::validation::SimpleConsensusValidationResult;

use dpp::fee::fee_result::FeeResult;

/// The Fee Result for a Dry Run (without state)
pub type EstimatedFeeResult = FeeResult;

/// An execution result
#[derive(Debug, Clone)]
pub enum EventExecutionResult {
    /// Successfully executed a paid event
    SuccessfulPaidExecution(EstimatedFeeResult, FeeResult),
    /// Successfully executed a free event
    SuccessfulFreeExecution,
    /// There were consensus errors when trying to execute an event
    ConsensusExecutionError(SimpleConsensusValidationResult),
}
