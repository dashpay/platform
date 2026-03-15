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
            StateTransitionExecutionResult::SuccessfulExecution {
                fee_result: actual_fees,
                ..
            } => Some(ExecTxResult {
                code: 0,
                gas_used: actual_fees.total_base_fee() as SignedCredits,
                ..Default::default()
            }),
            StateTransitionExecutionResult::UnpaidConsensusError(error) => Some(ExecTxResult {
                code: HandlerError::from(&error).code(),
                info: error.response_info_for_version(platform_version)?,
                gas_used: 0,
                ..Default::default()
            }),
            StateTransitionExecutionResult::PaidConsensusError { error, actual_fees } => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::state_transitions_processing_result::NotExecutedReason;
    use dpp::consensus::ConsensusError;
    use dpp::fee::fee_result::FeeResult;
    use std::collections::BTreeMap;

    #[test]
    fn successful_execution_produces_result_with_zero_code() {
        let platform_version = PlatformVersion::latest();

        let fee_result = FeeResult {
            storage_fee: 100,
            processing_fee: 50,
            ..Default::default()
        };

        let execution_result = StateTransitionExecutionResult::SuccessfulExecution {
            estimated_fees: None,
            fee_result: fee_result.clone(),
            address_balance_changes: BTreeMap::new(),
        };

        let result: Option<ExecTxResult> = execution_result
            .try_into_platform_versioned(platform_version)
            .expect("conversion should succeed");

        let tx_result = result.expect("should produce Some result");
        assert_eq!(tx_result.code, 0);
        assert_eq!(
            tx_result.gas_used,
            fee_result.total_base_fee() as SignedCredits
        );
    }

    #[test]
    fn unpaid_consensus_error_produces_result_with_error_code() {
        let platform_version = PlatformVersion::latest();

        let consensus_error = ConsensusError::DefaultError;
        let execution_result =
            StateTransitionExecutionResult::UnpaidConsensusError(consensus_error);

        let result: Option<ExecTxResult> = execution_result
            .try_into_platform_versioned(platform_version)
            .expect("conversion should succeed");

        let tx_result = result.expect("should produce Some result");
        assert_ne!(tx_result.code, 0);
        assert_eq!(tx_result.gas_used, 0);
        assert!(!tx_result.info.is_empty());
    }

    #[test]
    fn paid_consensus_error_produces_result_with_fees_and_error_code() {
        let platform_version = PlatformVersion::latest();

        let consensus_error = ConsensusError::DefaultError;
        let fee_result = FeeResult {
            storage_fee: 0,
            processing_fee: 200,
            ..Default::default()
        };

        let execution_result = StateTransitionExecutionResult::PaidConsensusError {
            error: consensus_error,
            actual_fees: fee_result.clone(),
        };

        let result: Option<ExecTxResult> = execution_result
            .try_into_platform_versioned(platform_version)
            .expect("conversion should succeed");

        let tx_result = result.expect("should produce Some result");
        assert_ne!(tx_result.code, 0);
        assert_eq!(
            tx_result.gas_used,
            fee_result.total_base_fee() as SignedCredits
        );
        assert!(!tx_result.info.is_empty());
    }

    #[test]
    fn internal_error_produces_result_with_internal_code() {
        let platform_version = PlatformVersion::latest();

        let execution_result =
            StateTransitionExecutionResult::InternalError("something broke".to_string());

        let result: Option<ExecTxResult> = execution_result
            .try_into_platform_versioned(platform_version)
            .expect("conversion should succeed");

        let tx_result = result.expect("should produce Some result");
        // Internal error code is 13
        assert_eq!(tx_result.code, 13);
        assert!(tx_result.info.is_empty());
    }

    #[test]
    fn not_executed_produces_none() {
        let platform_version = PlatformVersion::latest();

        let execution_result =
            StateTransitionExecutionResult::NotExecuted(NotExecutedReason::ProposerRanOutOfTime);

        let result: Option<ExecTxResult> = execution_result
            .try_into_platform_versioned(platform_version)
            .expect("conversion should succeed");

        assert!(result.is_none());
    }

    #[test]
    fn successful_execution_with_zero_fees() {
        let platform_version = PlatformVersion::latest();

        let fee_result = FeeResult::default();

        let execution_result = StateTransitionExecutionResult::SuccessfulExecution {
            estimated_fees: None,
            fee_result,
            address_balance_changes: BTreeMap::new(),
        };

        let result: Option<ExecTxResult> = execution_result
            .try_into_platform_versioned(platform_version)
            .expect("conversion should succeed");

        let tx_result = result.expect("should produce Some result");
        assert_eq!(tx_result.code, 0);
        assert_eq!(tx_result.gas_used, 0);
    }
}
