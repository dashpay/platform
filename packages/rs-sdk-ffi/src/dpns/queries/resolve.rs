//! Resolve DPNS names to identity IDs

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use serde_json::json;

/// Resolve a DPNS name to an identity ID
///
/// This function resolves a DPNS username to its associated identity ID.
/// The name can be either:
/// - A full domain name (e.g., "alice.dash")
/// - Just the label (e.g., "alice")
///
/// # Arguments
/// * `sdk_handle` - Handle to the SDK instance
/// * `name` - The DPNS name to resolve
///
/// # Returns
/// * On success: A JSON object with the identity ID, or null if not found
/// * On error: An error result
///
/// # Safety
/// - `sdk_handle` and `name` must be valid, non-null pointers.
/// - `name` must point to a NUL-terminated C string valid for the duration of the call.
/// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_resolve(
    sdk_handle: *const SDKHandle,
    name: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if name.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Name is null".to_string(),
        ));
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid UTF-8 in name".to_string(),
            ));
        }
    };

    if name_str.is_empty() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Name cannot be empty".to_string(),
        ));
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;

    // Execute the async operation
    let result = sdk_wrapper.runtime.block_on(async {
        match sdk.resolve_dpns_name_to_identity(name_str).await {
            Ok(Some(identity_id)) => {
                let response = json!({
                    "identityId": identity_id.to_string(Encoding::Base58)
                });
                Ok(response.to_string())
            }
            Ok(None) => {
                // Return an error instead of null for "not found"
                Err(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Name '{}' not found", name_str),
                ))
            }
            Err(e) => Err(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to resolve DPNS name: {}", e),
            )),
        }
    });

    match result {
        Ok(json) => match CString::new(json) {
            Ok(c_string) => DashSDKResult::success_string(c_string.into_raw()),
            Err(_) => DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                "Failed to convert JSON to C string".to_string(),
            )),
        },
        Err(e) => DashSDKResult::error(e),
    }
}
