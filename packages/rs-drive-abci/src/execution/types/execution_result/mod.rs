mod v0;

use dpp::errors::consensus::codes::ErrorWithCode;

use dpp::validation::SimpleConsensusValidationResult;

use dpp::fee::fee_result::FeeResult;
use dpp::fee::SignedCredits;
use tenderdash_abci::proto::abci::ExecTxResult;

/// The Fee Result for a Dry Run (without state)
pub type DryRunFeeResult = FeeResult;

/// An execution result
#[derive(Debug)]
pub(in crate::execution) enum ExecutionResult {
    /// Successfully executed a paid event
    SuccessfulPaidExecution(DryRunFeeResult, FeeResult),
    /// Successfully executed a free event
    SuccessfulFreeExecution,
    /// There were consensus errors when trying to execute an event
    ConsensusExecutionError(SimpleConsensusValidationResult),
}

// State transitions are never free, so we should filter out SuccessfulFreeExecution
// So we use an option
impl From<ExecutionResult> for ExecTxResult {
    fn from(value: ExecutionResult) -> Self {
        match value {
            ExecutionResult::SuccessfulPaidExecution(dry_run_fee_result, fee_result) => {
                ExecTxResult {
                    code: 0,
                    data: vec![],
                    log: "".to_string(),
                    info: "".to_string(),
                    gas_wanted: dry_run_fee_result.total_base_fee() as SignedCredits,
                    gas_used: fee_result.total_base_fee() as SignedCredits,
                    events: vec![],
                    codespace: "".to_string(),
                }
            }
            ExecutionResult::SuccessfulFreeExecution => ExecTxResult {
                code: 0,
                data: vec![],
                log: "".to_string(),
                info: "".to_string(),
                gas_wanted: 0,
                gas_used: 0,
                events: vec![],
                codespace: "".to_string(),
            },
            ExecutionResult::ConsensusExecutionError(validation_result) => {
                let code = validation_result
                    .errors
                    .first()
                    .map(|error| error.code())
                    .unwrap_or(1);
                ExecTxResult {
                    code,
                    data: vec![],
                    log: "".to_string(),
                    info: "".to_string(),
                    gas_wanted: 0,
                    gas_used: 0,
                    events: vec![],
                    codespace: "".to_string(),
                }
            }
        }
    }
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
