//! Search DPNS names by prefix

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use serde_json::json;
use std::ffi::CString;

/// Search for DPNS names that start with a given prefix
///
/// This function searches for DPNS usernames that start with the given prefix.
///
/// # Arguments
/// * `sdk_handle` - Handle to the SDK instance
/// * `prefix` - The prefix to search for (e.g., "ali" to find "alice", "alicia", etc.)
/// * `limit` - Maximum number of results to return (0 for default of 10)
///
/// # Returns
/// * On success: A JSON array of username objects
/// * On error: An error result
///
/// # Safety
/// - `sdk_handle` and `prefix` must be valid, non-null pointers.
/// - `prefix` must point to a NUL-terminated C string valid for the duration of the call.
/// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_search(
    sdk_handle: *const SDKHandle,
    prefix: *const c_char,
    limit: u32,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if prefix.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Prefix is null".to_string(),
        ));
    }

    let prefix_str = match CStr::from_ptr(prefix).to_str() {
        Ok(s) => s,
        Err(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid UTF-8 in prefix".to_string(),
            ));
        }
    };

    if prefix_str.is_empty() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Prefix cannot be empty".to_string(),
        ));
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;

    let limit_opt = if limit == 0 { None } else { Some(limit) };

    // Execute the async operation
    let result = sdk_wrapper.runtime.block_on(async {
        match sdk.search_dpns_names(prefix_str, limit_opt).await {
            Ok(usernames) => {
                // Convert to JSON array
                let json_array: Vec<serde_json::Value> = usernames
                    .into_iter()
                    .map(|username| {
                        let mut obj = json!({
                            "label": username.label,
                            "normalizedLabel": username.normalized_label,
                            "fullName": username.full_name,
                            "ownerId": username.owner_id.to_string(Encoding::Base58)
                        });

                        if let Some(id) = username.records_identity_id {
                            obj["recordsIdentityId"] = json!(id.to_string(Encoding::Base58));
                        }

                        obj
                    })
                    .collect();

                Ok(json!(json_array).to_string())
            }
            Err(e) => Err(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to search DPNS names: {}", e),
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
