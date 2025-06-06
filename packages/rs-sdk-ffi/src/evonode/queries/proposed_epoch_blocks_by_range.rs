use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKResult, FFIError};
use dash_sdk::platform::FetchMany;
use dashcore_rpc::dashcore::ProTxHash;
use drive_proof_verifier::types::{ProposerBlockCountByRange, ProposerBlockCounts};
use std::ffi::{c_char, CStr, CString};

/// Fetches proposed epoch blocks by range
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `epoch` - Epoch number (optional, 0 for current epoch)
/// * `limit` - Maximum number of results to return (optional, 0 for no limit)
/// * `start_after` - Start after this pro_tx_hash (hex-encoded, optional)
/// * `start_at` - Start at this pro_tx_hash (hex-encoded, optional)
///
/// # Returns
/// * JSON array of evonode proposed block counts or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_evonode_get_proposed_epoch_blocks_by_range(
    sdk_handle: *const SDKHandle,
    epoch: u32,
    limit: u32,
    start_after: *const c_char,
    start_at: *const c_char,
) -> DashSDKResult {
    match get_evonodes_proposed_epoch_blocks_by_range(
        sdk_handle,
        epoch,
        limit,
        start_after,
        start_at,
    ) {
        Ok(Some(json)) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data: std::ptr::null(),
                        error: DashSDKError::new(&format!("Failed to create CString: {}", e)),
                    }
                }
            };
            DashSDKResult {
                data: c_str.into_raw(),
                error: std::ptr::null(),
            }
        }
        Ok(None) => DashSDKResult {
            data: std::ptr::null(),
            error: std::ptr::null(),
        },
        Err(e) => DashSDKResult {
            data: std::ptr::null(),
            error: DashSDKError::new(&e),
        },
    }
}

fn get_evonodes_proposed_epoch_blocks_by_range(
    sdk_handle: *const SDKHandle,
    epoch: u32,
    limit: u32,
    start_after: *const c_char,
    start_at: *const c_char,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let sdk = unsafe { &*sdk_handle }.sdk.clone();

    rt.block_on(async move {
        let start_after_hash = if start_after.is_null() {
            None
        } else {
            let start_after_str = unsafe {
                CStr::from_ptr(start_after)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in start_after: {}", e))?
            };
            let bytes = hex::decode(start_after_str)
                .map_err(|e| format!("Failed to decode start_after: {}", e))?;
            let hash_bytes: [u8; 32] = bytes
                .try_into()
                .map_err(|_| "start_after must be exactly 32 bytes".to_string())?;
            Some(ProTxHash::from(hash_bytes))
        };

        let start_at_hash = if start_at.is_null() {
            None
        } else {
            let start_at_str = unsafe {
                CStr::from_ptr(start_at)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in start_at: {}", e))?
            };
            let bytes = hex::decode(start_at_str)
                .map_err(|e| format!("Failed to decode start_at: {}", e))?;
            let hash_bytes: [u8; 32] = bytes
                .try_into()
                .map_err(|_| "start_at must be exactly 32 bytes".to_string())?;
            Some(ProTxHash::from(hash_bytes))
        };

        // Create a query with the epoch and range parameters
        let query = dash_sdk::platform::LimitQuery {
            query: EvonodesProposedEpochBlocksByRangeQuery {
                epoch: if epoch > 0 { Some(epoch) } else { None },
                start_after: start_after_hash,
                start_at: start_at_hash,
            },
            limit: if limit > 0 { Some(limit) } else { None },
            start_info: None,
        };

        match ProposerBlockCountByRange::fetch_many(&sdk, query).await {
            Ok(block_counts) => {
                if block_counts.is_empty() {
                    return Ok(None);
                }

                let block_counts_json: Vec<String> = block_counts
                    .iter()
                    .map(|(pro_tx_hash, count)| {
                        format!(
                            r#"{{"pro_tx_hash":"{}","count":{}}}"#,
                            hex::encode(pro_tx_hash.to_byte_array()),
                            count
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", block_counts_json.join(","))))
            }
            Err(e) => Err(format!(
                "Failed to fetch evonodes proposed epoch blocks by range: {}",
                e
            )),
        }
    })
}

// Helper struct for the query
#[derive(Debug, Clone)]
struct EvonodesProposedEpochBlocksByRangeQuery {
    pub epoch: Option<u32>,
    pub start_after: Option<ProTxHash>,
    pub start_at: Option<ProTxHash>,
}

impl
    dash_sdk::platform::Query<dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksByRangeRequest>
    for EvonodesProposedEpochBlocksByRangeQuery
{
    fn query(
        self,
        prove: bool,
    ) -> Result<
        dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksByRangeRequest,
        dash_sdk::Error,
    > {
        use dapi_grpc::platform::v0::{
            get_evonodes_proposed_epoch_blocks_by_range_request::{
                get_evonodes_proposed_epoch_blocks_by_range_request_v0::Start,
                GetEvonodesProposedEpochBlocksByRangeRequestV0, Version,
            },
            GetEvonodesProposedEpochBlocksByRangeRequest,
        };

        let start = if let Some(start_after) = self.start_after {
            Some(Start::StartAfter(start_after.to_byte_array().to_vec()))
        } else if let Some(start_at) = self.start_at {
            Some(Start::StartAt(start_at.to_byte_array().to_vec()))
        } else {
            None
        };

        let request = GetEvonodesProposedEpochBlocksByRangeRequest {
            version: Some(Version::V0(
                GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                    epoch: self.epoch,
                    limit: None, // Limit is handled by LimitQuery wrapper
                    start,
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_evonodes_proposed_epoch_blocks_by_range_null_handle() {
        unsafe {
            let result = dash_sdk_evonode_get_proposed_epoch_blocks_by_range(
                std::ptr::null(),
                0,
                10,
                std::ptr::null(),
                std::ptr::null(),
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_evonodes_proposed_epoch_blocks_by_range() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_evonode_get_proposed_epoch_blocks_by_range(
                handle,
                0,
                10,
                std::ptr::null(),
                std::ptr::null(),
            );
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
