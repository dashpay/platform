use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKResult, FFIError};
use dash_sdk::platform::FetchUnproved;
use drive_proof_verifier::types::CurrentQuorumsInfo;
use std::ffi::CString;
use std::os::raw::c_char;

/// Fetches information about current quorums
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
///
/// # Returns
/// * JSON string with current quorums information
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_system_get_current_quorums_info(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    match get_current_quorums_info(sdk_handle) {
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

fn get_current_quorums_info(sdk_handle: *const SDKHandle) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let sdk = unsafe { &*sdk_handle }.sdk.clone();

    rt.block_on(async move {
        match CurrentQuorumsInfo::fetch_unproved(&sdk, ()).await {
            Ok(Some(info)) => {
                // Convert quorum hashes to hex strings
                let quorum_hashes_json: Vec<String> = info
                    .quorum_hashes
                    .iter()
                    .map(|hash| format!("\"{}\"", hex::encode(hash)))
                    .collect();

                // Convert validator sets to JSON
                let validator_sets_json: Vec<String> = info
                    .validator_sets
                    .iter()
                    .map(|vs| {
                        let members_json: Vec<String> = vs
                            .members
                            .iter()
                            .map(|m| {
                                format!(
                                    r#"{{"pro_tx_hash":"{}","node_ip":"{}","is_banned":{}}}"#,
                                    hex::encode(&m.pro_tx_hash),
                                    m.node_ip,
                                    m.is_banned
                                )
                            })
                            .collect();

                        format!(
                            r#"{{"quorum_hash":"{}","core_height":{},"members":[{}],"threshold_public_key":"{}"}}"#,
                            hex::encode(&vs.quorum_hash),
                            vs.core_height,
                            members_json.join(","),
                            hex::encode(&vs.threshold_public_key)
                        )
                    })
                    .collect();

                let json = format!(
                    r#"{{"quorum_hashes":[{}],"current_quorum_hash":"{}","validator_sets":[{}],"last_block_proposer":"{}","last_platform_block_height":{}}}"#,
                    quorum_hashes_json.join(","),
                    hex::encode(&info.current_quorum_hash),
                    validator_sets_json.join(","),
                    hex::encode(&info.last_block_proposer),
                    info.last_platform_block_height
                );

                Ok(Some(json))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to fetch current quorums info: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_current_quorums_info_null_handle() {
        unsafe {
            let result = dash_sdk_system_get_current_quorums_info(std::ptr::null());
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_current_quorums_info() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_system_get_current_quorums_info(handle);
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
