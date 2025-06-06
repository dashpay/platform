use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::version::ProtocolVersionVoteCount;
use dash_sdk::platform::FetchMany;
use std::ffi::CString;
use std::os::raw::c_void;

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

fn get_protocol_version_upgrade_state(
    sdk_handle: *const SDKHandle,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        match ProtocolVersionVoteCount::fetch_many(&sdk, ()).await {
            Ok(upgrades) => {
                let upgrades: dash_sdk::query_types::ProtocolVersionUpgrades = upgrades;
                if upgrades.is_empty() {
                    return Ok(None);
                }

                let upgrades_json: Vec<String> = upgrades
                    .iter()
                    .filter_map(|(version, vote_count_opt)| {
                        vote_count_opt.as_ref().map(|vote_count| {
                            format!(
                                r#"{{"version_number":{},"vote_count":{}}}"#,
                                version, vote_count
                            )
                        })
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
