use crate::abci::handler::error::consensus::AbciResponseInfoGetter;
use crate::abci::handler::error::HandlerError;
use crate::error::Error;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use dpp::fee::SignedCredits;
use dpp::version::PlatformVersion;
use dpp::version::TryIntoPlatformVersioned;
use tenderdash_abci::proto::abci::ExecTxResult;

impl TryIntoPlatformVersioned<Option<ExecTxResult>> for StateTransitionExecutionResult {
    type Error = Error;

    fn try_into_platform_versioned(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ExecTxResult>, Self::Error> {
        let response = match self {
            StateTransitionExecutionResult::SuccessfulExecution(_, actual_fees) => {
                Some(ExecTxResult {
                    code: 0,
                    gas_used: actual_fees.total_base_fee() as SignedCredits,
                    ..Default::default()
                })
            }
            StateTransitionExecutionResult::UnpaidConsensusError(error) => Some(ExecTxResult {
                code: HandlerError::from(&error).code(),
                info: error.response_info_for_version(platform_version)?,
                gas_used: 0,
                ..Default::default()
            }),
            StateTransitionExecutionResult::PaidConsensusError(error, actual_fees) => {
                Some(ExecTxResult {
                    code: HandlerError::from(&error).code(),
                    info: error.response_info_for_version(platform_version)?,
                    gas_used: actual_fees.total_base_fee() as SignedCredits,
                    ..Default::default()
                })
            }
            StateTransitionExecutionResult::InternalError(message) => Some(ExecTxResult {
                code: HandlerError::Internal(message).code(),
                // TODO: That would be nice to provide more information about the error for debugging
                info: String::default(),
                ..Default::default()
            }),
            StateTransitionExecutionResult::NotExecuted(_) => None,
        };

        Ok(response)
    }
}
