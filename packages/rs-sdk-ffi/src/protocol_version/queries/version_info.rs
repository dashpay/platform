use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use std::ffi::CString;

/// Read cached protocol version information without performing network I/O.
/// Returns JSON with verifiedVersion (nullable), effectiveVersion,
/// maximumSupportedVersion and newerProtocolAvailable. Ordinary proved queries
/// populate the observation, including for SDKs with a pinned operation version.
///
/// # Safety
/// `sdk_handle` must be a live SDK handle. Free the returned string with
/// `dash_sdk_string_free`, or the returned error with `dash_sdk_error_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_protocol_version_info(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let info = wrapper.sdk.protocol_version_info();
    let json = serde_json::json!({
        "verifiedVersion": info.verified_version,
        "effectiveVersion": info.effective_version,
        "maximumSupportedVersion": info.maximum_supported_version,
        "newerProtocolAvailable": info.newer_protocol_available,
    });
    match CString::new(json.to_string()) {
        Ok(value) => DashSDKResult::success_string(value.into_raw()),
        Err(error) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            error.to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use std::ffi::CStr;

    #[test]
    fn protocol_version_info_null_handle() {
        unsafe {
            let result = dash_sdk_get_protocol_version_info(std::ptr::null());
            assert!(!result.error.is_null());
            assert_eq!((*result.error).code, DashSDKErrorCode::InvalidParameter);
            crate::dash_sdk_error_free(result.error);
        }
    }

    #[test]
    fn protocol_version_info_json_and_ownership() {
        unsafe {
            let handle = create_mock_sdk_handle();
            let result = dash_sdk_get_protocol_version_info(handle);
            assert!(result.error.is_null());
            let string = result.data.cast::<std::os::raw::c_char>();
            let json: serde_json::Value =
                serde_json::from_slice(CStr::from_ptr(string).to_bytes()).unwrap();
            assert!(json["verifiedVersion"].is_null());
            assert_eq!(json["newerProtocolAvailable"], false);
            assert!(json["effectiveVersion"].is_u64());
            assert!(json["maximumSupportedVersion"].is_u64());
            crate::dash_sdk_string_free(string);
            destroy_mock_sdk_handle(handle);
        }
    }
}
