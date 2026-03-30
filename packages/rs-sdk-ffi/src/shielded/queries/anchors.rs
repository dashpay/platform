use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::query_types::ShieldedAnchors;
use std::ffi::CString;
use std::os::raw::c_void;

/// Fetches all anchors from the shielded pool.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
///
/// # Returns
/// * JSON array of hex-encoded 32-byte anchor hashes
/// * Error if the operation fails
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - On success, returns a heap-allocated C string (JSON); caller must free with `dash_sdk_string_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_get_anchors(
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
        ShieldedAnchors::fetch_current(&sdk)
            .await
            .map_err(|e| format!("Failed to fetch shielded anchors: {}", e))
    });

    match result {
        Ok(ShieldedAnchors(anchors)) => {
            let json_anchors: Vec<String> = anchors.iter().map(hex::encode).collect();

            let json_str =
                serde_json::to_string(&json_anchors).unwrap_or_else(|_| "[]".to_string());

            match CString::new(json_str) {
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
        Err(e) => DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e)),
    }
}
