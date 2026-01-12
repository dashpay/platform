use crate::abci::handler::error::consensus::AbciResponseInfoGetter;
use crate::abci::handler::error::HandlerError;
use crate::error::Error;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::SignedCredits;
use dpp::version::PlatformVersion;
use dpp::version::TryIntoPlatformVersioned;
use tenderdash_abci::proto::abci::ExecTxResult;

/// Encodes fee result into a byte vector that can be stored in ExecTxResult.data
/// Format: processing_fee (8 bytes LE) | storage_fee (8 bytes LE) | removed_bytes_from_system (4 bytes LE)
fn encode_fee_result(fee_result: &FeeResult) -> Vec<u8> {
    let mut data = Vec::with_capacity(20);
    data.extend_from_slice(&fee_result.processing_fee.to_le_bytes());
    data.extend_from_slice(&fee_result.storage_fee.to_le_bytes());
    data.extend_from_slice(&fee_result.removed_bytes_from_system.to_le_bytes());
    data
}

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
                data: encode_fee_result(&actual_fees),
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
                    data: encode_fee_result(&actual_fees),
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

    #[test]
    fn test_encode_fee_result() {
        let mut fee_result = FeeResult::default();
        fee_result.processing_fee = 1234567890123456789;
        fee_result.storage_fee = 9876543210987654321;
        fee_result.removed_bytes_from_system = 42;

        let encoded = encode_fee_result(&fee_result);

        // Verify the encoding is correct
        assert_eq!(encoded.len(), 20);

        // Decode and verify
        let decoded_processing = u64::from_le_bytes(encoded[0..8].try_into().unwrap());
        let decoded_storage = u64::from_le_bytes(encoded[8..16].try_into().unwrap());
        let decoded_removed = u32::from_le_bytes(encoded[16..20].try_into().unwrap());

        assert_eq!(decoded_processing, fee_result.processing_fee);
        assert_eq!(decoded_storage, fee_result.storage_fee);
        assert_eq!(decoded_removed, fee_result.removed_bytes_from_system);
    }

    #[test]
    fn test_encode_fee_result_zero_values() {
        let fee_result = FeeResult::default();

        let encoded = encode_fee_result(&fee_result);

        assert_eq!(encoded.len(), 20);
        assert!(encoded.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_encode_fee_result_max_values() {
        let mut fee_result = FeeResult::default();
        fee_result.processing_fee = u64::MAX;
        fee_result.storage_fee = u64::MAX;
        fee_result.removed_bytes_from_system = u32::MAX;

        let encoded = encode_fee_result(&fee_result);

        assert_eq!(encoded.len(), 20);

        let decoded_processing = u64::from_le_bytes(encoded[0..8].try_into().unwrap());
        let decoded_storage = u64::from_le_bytes(encoded[8..16].try_into().unwrap());
        let decoded_removed = u32::from_le_bytes(encoded[16..20].try_into().unwrap());

        assert_eq!(decoded_processing, u64::MAX);
        assert_eq!(decoded_storage, u64::MAX);
        assert_eq!(decoded_removed, u32::MAX);
    }
}
