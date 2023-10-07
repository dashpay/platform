use crate::abci::handler::error::consensus::AbciResponseInfoGetter;
use crate::error::Error;
use crate::platform_types::state_transition_execution_result::StateTransitionExecutionResult;
use dpp::consensus::codes::ErrorWithCode;
use dpp::fee::SignedCredits;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci::ExecTxResult;

// State transitions are never free, so we should filter out SuccessfulFreeExecution
// So we use an option
impl StateTransitionExecutionResult {
    /// Convert state transition execution result into a Tenderdash tx result
    pub fn try_into_tx_result(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<ExecTxResult, Error> {
        let response = match self {
            Self::SuccessfulPaidExecution(dry_run_fee_result, fee_result) => ExecTxResult {
                code: 0,
                data: vec![],
                log: "".to_string(),
                info: "".to_string(),
                gas_wanted: dry_run_fee_result.total_base_fee() as SignedCredits,
                gas_used: fee_result.total_base_fee() as SignedCredits,
                events: vec![],
                codespace: "".to_string(),
            },
            Self::SuccessfulFreeExecution => ExecTxResult {
                code: 0,
                data: vec![],
                log: "".to_string(),
                info: "".to_string(),
                gas_wanted: 0,
                gas_used: 0,
                events: vec![],
                codespace: "".to_string(),
            },
            Self::ConsensusExecutionError(validation_result) => {
                let error = validation_result
                    .errors
                    .first()
                    .expect("invalid execution result should have a consensus error");

                ExecTxResult {
                    code: error.code(),
                    data: vec![],
                    log: "".to_string(),
                    info: error.response_info_for_version(platform_version)?,
                    gas_wanted: 0,
                    gas_used: 0,
                    events: vec![],
                    codespace: "".to_string(),
                }
            }
        };

        Ok(response)
    }
}
