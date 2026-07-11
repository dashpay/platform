use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType, FFIError};
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
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - The function does not retain references to the input pointer beyond the duration of the call.
/// - On success, the returned `DashSDKResult` may contain a heap-allocated C string; the caller must
///   free it using the SDK's free routine to avoid leaks. It may also return no data (null pointer).
/// - Passing a dangling or invalid pointer for `sdk_handle` results in undefined behavior.
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
                        data_type: DashSDKResultDataType::NoData,
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
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        },
        // Preserve the typed `dash_sdk::Error` classification (network /
        // timeout / etc.) instead of flattening everything to InternalError.
        // A transient DAPI transport failure (e.g. an evonode serving an
        // expired TLS certificate) is a NetworkError, not an internal bug, and
        // the UI should label it as such rather than "Internal Error".
        Err(e) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(DashSDKError::from(e))),
        },
    }
}

fn get_protocol_version_upgrade_state(
    sdk_handle: *const SDKHandle,
) -> Result<Option<String>, FFIError> {
    if sdk_handle.is_null() {
        return Err(FFIError::InvalidState("SDK handle is null".to_string()));
    }

    let rt = crate::runtime::BigStackRuntime::new_isolated()
        .map_err(|e| FFIError::InternalError(format!("Failed to create Tokio runtime: {}", e)))?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        // Propagate the `dash_sdk::Error` unchanged via `?` so the FFI error
        // converter classifies it (NetworkError, Timeout, ...). Returning a
        // pre-formatted String here would erase that classification.
        let upgrades: dash_sdk::query_types::ProtocolVersionUpgrades =
            ProtocolVersionVoteCount::fetch_many(&sdk, ()).await?;

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
            let _result = dash_sdk_protocol_version_get_upgrade_state(handle);
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
