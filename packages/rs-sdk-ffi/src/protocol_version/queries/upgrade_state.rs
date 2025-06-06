use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKResult, FFIError};
use dash_sdk::platform::FetchMany;
use dpp::version::ProtocolVersionVoteCount;
use drive_proof_verifier::types::ProtocolVersionUpgrades;
use std::ffi::CString;
use std::os::raw::c_char;

/// Fetches protocol version upgrade state
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
///
/// # Returns
/// * JSON array of protocol version upgrade information
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_protocol_version_get_upgrade_state(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    match get_protocol_version_upgrade_state(sdk_handle) {
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

fn get_protocol_version_upgrade_state(
    sdk_handle: *const SDKHandle,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let sdk = unsafe { &*sdk_handle }.sdk.clone();

    rt.block_on(async move {
        match ProtocolVersionVoteCount::fetch_many(&sdk, ()).await {
            Ok(upgrades) => {
                if upgrades.is_empty() {
                    return Ok(None);
                }

                let upgrades_json: Vec<String> = upgrades
                    .iter()
                    .map(|(version, vote_count)| {
                        format!(
                            r#"{{"version_number":{},"vote_count":{}}}"#,
                            version,
                            vote_count.vote_count()
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", upgrades_json.join(","))))
            }
            Err(e) => Err(format!(
                "Failed to fetch protocol version upgrade state: {}",
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
    fn test_get_protocol_version_upgrade_state_null_handle() {
        unsafe {
            let result = dash_sdk_protocol_version_get_upgrade_state(std::ptr::null());
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_protocol_version_upgrade_state() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_protocol_version_get_upgrade_state(handle);
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
