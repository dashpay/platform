use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType, FFIError};
use dash_sdk::dashcore_rpc::dashcore::ProTxHash;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::{MasternodeProtocolVote, MasternodeProtocolVotes};
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches protocol version upgrade vote status
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `start_pro_tx_hash` - Starting masternode pro_tx_hash (hex-encoded, optional)
/// * `count` - Number of vote entries to retrieve
///
/// # Returns
/// * JSON array of masternode protocol version votes or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_protocol_version_get_upgrade_vote_status(
    sdk_handle: *const SDKHandle,
    start_pro_tx_hash: *const c_char,
    count: u32,
) -> DashSDKResult {
    match get_protocol_version_upgrade_vote_status(sdk_handle, start_pro_tx_hash, count) {
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

fn get_protocol_version_upgrade_vote_status(
    sdk_handle: *const SDKHandle,
    start_pro_tx_hash: *const c_char,
    count: u32,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        let start_hash = if start_pro_tx_hash.is_null() {
            None
        } else {
            let start_hash_str = unsafe {
                CStr::from_ptr(start_pro_tx_hash)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in start pro_tx_hash: {}", e))?
            };
            let bytes = hex::decode(start_hash_str)
                .map_err(|e| format!("Failed to decode start pro_tx_hash: {}", e))?;
            let hash_bytes: [u8; 32] = bytes
                .try_into()
                .map_err(|_| "start_pro_tx_hash must be exactly 32 bytes".to_string())?;
            Some(ProTxHash::from(hash_bytes))
        };

        let query = dash_sdk::platform::LimitQuery {
            query: start_hash,
            limit: Some(count),
            start_info: None,
        };

        match MasternodeProtocolVote::fetch_many(&sdk, query).await {
            Ok(votes) => {
                if votes.is_empty() {
                    return Ok(None);
                }

                let votes_json: Vec<String> = votes
                    .iter()
                    .filter_map(|(pro_tx_hash, vote_opt)| {
                        vote_opt.as_ref().map(|vote| {
                            format!(
                                r#"{{"pro_tx_hash":"{}","version":{}}}"#,
                                hex::encode(pro_tx_hash),
                                vote.voted_version
                            )
                        })
                    })
                    .collect();

                Ok(Some(format!("[{}]", votes_json.join(","))))
            }
            Err(e) => Err(format!(
                "Failed to fetch protocol version upgrade vote status: {}",
                e
            )),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_protocol_version_upgrade_vote_status_null_handle() {
        unsafe {
            let result = dash_sdk_protocol_version_get_upgrade_vote_status(
                std::ptr::null(),
                std::ptr::null(),
                10,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_protocol_version_upgrade_vote_status() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result =
                dash_sdk_protocol_version_get_upgrade_vote_status(handle, std::ptr::null(), 10);
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
