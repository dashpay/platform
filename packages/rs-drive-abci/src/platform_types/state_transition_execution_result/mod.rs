use dpp::validation::SimpleConsensusValidationResult;

use dpp::fee::fee_result::FeeResult;

/// The Fee Result for a Dry Run (without state)
pub type DryRunFeeResult = FeeResult;

/// An execution result
#[derive(Debug, Clone)]
pub enum StateTransitionExecutionResult {
    /// Successfully executed a paid event
    SuccessfulPaidExecution(DryRunFeeResult, FeeResult),
    /// Successfully executed a free event
    SuccessfulFreeExecution,
    /// There were consensus errors when trying to execute an event
    ConsensusExecutionError(SimpleConsensusValidationResult),
}

// impl From<ValidationResult<ExecutionResult>> for ExecutionResult {
//     fn from(value: ValidationResult<ExecutionResult>) -> Self {
//         let ValidationResult { errors, data } = value;
//         if let Some(result) = data {
//             if !errors.is_empty() {
//                 ConsensusExecutionError(SimpleValidationResult::new_with_errors(errors))
//             } else {
//                 result
//             }
//         } else {
//             ConsensusExecutionError(SimpleValidationResult::new_with_errors(errors))
//         }
//     }
// }
