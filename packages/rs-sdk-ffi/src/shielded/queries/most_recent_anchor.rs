use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::{MostRecentShieldedAnchor, NoParamQuery};
use std::ffi::CString;
use std::os::raw::c_void;

/// Fetches the most recent anchor from the shielded pool.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
///
/// # Returns
/// * Hex-encoded string of the 32-byte anchor hash
/// * Error if the operation fails
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - On success, returns a heap-allocated C string; caller must free with `dash_sdk_string_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_get_most_recent_anchor(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let sdk = wrapper.sdk.clone();

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create Tokio runtime: {}", e),
            ));
        }
    };

    let result = rt.block_on(async move {
        MostRecentShieldedAnchor::fetch(&sdk, NoParamQuery {})
            .await
            .map_err(|e| format!("Failed to fetch most recent shielded anchor: {}", e))
    });

    match result {
        Ok(Some(MostRecentShieldedAnchor(anchor))) => {
            let hex_str = hex::encode(anchor);

            match CString::new(hex_str) {
                Ok(c_str) => DashSDKResult {
                    data_type: DashSDKResultDataType::String,
                    data: c_str.into_raw() as *mut c_void,
                    error: std::ptr::null_mut(),
                },
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Failed to create CString: {}", e),
                )),
            }
        }
        Ok(None) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            "No anchor found in shielded pool".to_string(),
        )),
        Err(e) => DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e)),
    }
}
