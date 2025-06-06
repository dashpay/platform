use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dashcore_rpc::dashcore::ProTxHash;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::ProposerBlockCountById;
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches proposed epoch blocks by evonode IDs
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `epoch` - Epoch number (optional, 0 for current epoch)
/// * `ids_json` - JSON array of hex-encoded evonode pro_tx_hash IDs
///
/// # Returns
/// * JSON array of evonode proposed block counts or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_evonode_get_proposed_epoch_blocks_by_ids(
    sdk_handle: *const SDKHandle,
    epoch: u32,
    ids_json: *const c_char,
) -> DashSDKResult {
    match get_evonodes_proposed_epoch_blocks_by_ids(sdk_handle, epoch, ids_json) {
        Ok(Some(json)) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data_type: DashSDKResultDataType::None,
                        data: std::ptr::null_mut(),
                        error: Box::into_raw(Box::new(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            format!("Failed to create CString: {}", e),
                        ))),
                    }
                }
            };
            DashSDKResult {
                data_type: DashSDKResultDataType::String,
                data: c_str.into_raw() as *mut c_void,
                error: std::ptr::null_mut(),
            }
        }
        Ok(None) => DashSDKResult {
            data_type: DashSDKResultDataType::None,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        },
        Err(e) => DashSDKResult {
            data_type: DashSDKResultDataType::None,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                e,
            ))),
        },
    }
}

fn get_evonodes_proposed_epoch_blocks_by_ids(
    sdk_handle: *const SDKHandle,
    epoch: u32,
    ids_json: *const c_char,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let ids_str = unsafe {
        CStr::from_ptr(ids_json)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in IDs: {}", e))?
    };
    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        // Parse IDs JSON array
        let ids_array: Vec<String> = serde_json::from_str(ids_str)
            .map_err(|e| format!("Failed to parse IDs JSON: {}", e))?;

        let pro_tx_hashes: Result<Vec<ProTxHash>, String> = ids_array
            .into_iter()
            .map(|hex_str| {
                let bytes = hex::decode(&hex_str)
                    .map_err(|e| format!("Failed to decode pro_tx_hash: {}", e))?;
                let hash_bytes: [u8; 32] = bytes
                    .try_into()
                    .map_err(|_| "Pro_tx_hash must be exactly 32 bytes".to_string())?;
                Ok(ProTxHash::from(hash_bytes))
            })
            .collect();

        let pro_tx_hashes = pro_tx_hashes?;

        // Create a query with the epoch and pro_tx_hashes
        let query = EvonodesProposedEpochBlocksByIdsQuery {
            epoch: if epoch > 0 { Some(epoch) } else { None },
            pro_tx_hashes,
        };

        match ProposerBlockCountById::fetch_many(&sdk, query).await {
            Ok(block_counts) => {
                if block_counts.0.is_empty() {
                    return Ok(None);
                }

                let block_counts_json: Vec<String> = block_counts
                    .0
                    .iter()
                    .map(|(pro_tx_hash, count)| {
                        format!(
                            r#"{{"pro_tx_hash":"{}","count":{}}}"#,
                            hex::encode(&pro_tx_hash),
                            count
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", block_counts_json.join(","))))
            }
            Err(e) => Err(format!(
                "Failed to fetch evonodes proposed epoch blocks by IDs: {}",
                e
            )),
        }
    })
}

// Helper struct for the query
#[derive(Debug, Clone)]
struct EvonodesProposedEpochBlocksByIdsQuery {
    pub epoch: Option<u32>,
    pub pro_tx_hashes: Vec<ProTxHash>,
}

impl
    dash_sdk::platform::Query<
        dash_sdk::dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksByIdsRequest,
    > for EvonodesProposedEpochBlocksByIdsQuery
{
    fn query(
        self,
        prove: bool,
    ) -> Result<
        dash_sdk::dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksByIdsRequest,
        dash_sdk::Error,
    > {
        use dash_sdk::dapi_grpc::platform::v0::{
            get_evonodes_proposed_epoch_blocks_by_ids_request::{
                GetEvonodesProposedEpochBlocksByIdsRequestV0, Version,
            },
        };

        let request =
            dash_sdk::dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksByIdsRequest {
                version: Some(Version::V0(GetEvonodesProposedEpochBlocksByIdsRequestV0 {
                    epoch: self.epoch,
                    ids: self
                        .pro_tx_hashes
                        .into_iter()
                        .map(|hash| AsRef::<[u8]>::as_ref(&hash).to_vec())
                        .collect(),
                    prove,
                })),
            };

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_evonodes_proposed_epoch_blocks_by_ids_null_handle() {
        unsafe {
            let result = dash_sdk_evonode_get_proposed_epoch_blocks_by_ids(
                std::ptr::null(),
                0,
                CString::new(
                    r#"["0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"]"#,
                )
                .unwrap()
                .as_ptr(),
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_evonodes_proposed_epoch_blocks_by_ids_null_ids() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result =
                dash_sdk_evonode_get_proposed_epoch_blocks_by_ids(handle, 0, std::ptr::null());
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
