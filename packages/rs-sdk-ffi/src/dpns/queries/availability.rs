//! Check DPNS name availability

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::platform::dpns_usernames::is_valid_username;
use serde_json::json;
use std::ffi::CString;

/// Check if a DPNS username is available
///
/// This function checks if a given username is available for registration.
/// It also validates the username format and checks if it's contested.
///
/// # Arguments
/// * `sdk_handle` - Handle to the SDK instance
/// * `label` - The username label to check (e.g., "alice")
///
/// # Returns
/// * On success: A JSON object with availability information
/// * On error: An error result
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_check_availability(
    sdk_handle: *const SDKHandle,
    label: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if label.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Label is null".to_string(),
        ));
    }

    let label_str = match CStr::from_ptr(label).to_str() {
        Ok(s) => s,
        Err(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid UTF-8 in label".to_string(),
            ));
        }
    };

    // First check if the username is valid
    let is_valid = is_valid_username(label_str);
    if !is_valid {
        let result = json!({
            "label": label_str,
            "valid": false,
            "available": false,
            "message": "❌ Invalid username format",
            "requirements": [
                "Must be 3-63 characters long",
                "Must start and end with a letter or number",
                "Can only contain letters, numbers, and hyphens",
                "Cannot have consecutive hyphens"
            ]
        });
        match CString::new(result.to_string()) {
            Ok(c_string) => return DashSDKResult::success_string(c_string.into_raw()),
            Err(_) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    "Failed to convert JSON to C string".to_string(),
                ))
            }
        }
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;

    // Check homograph safety
    use dash_sdk::platform::dpns_usernames::{
        convert_to_homograph_safe_chars, is_contested_username,
    };
    let homograph_safe = convert_to_homograph_safe_chars(label_str);
    let is_homograph_different = homograph_safe != label_str.to_lowercase();
    let is_contested = is_contested_username(label_str);

    // Execute the async operation
    let result = sdk_wrapper.runtime.block_on(async {
        match sdk.check_dpns_name_availability(label_str).await {
            Ok(is_available) => {
                let mut result = json!({
                    "label": label_str,
                    "valid": true,
                    "available": is_available,
                    "normalizedLabel": homograph_safe,
                    "isContested": is_contested
                });

                if is_available {
                    result["message"] = json!("✅ Username is available");
                } else {
                    result["message"] = json!("❌ Username is already taken");
                }

                if is_homograph_different {
                    result["note"] = json!(format!("Note: Your username will be stored as \"{}\" to prevent homograph attacks", homograph_safe));
                }

                if is_contested && is_available {
                    result["contestedNote"] = json!("⚠️ This is a contested username (3-19 chars, only a-z/0/1/-). It requires masternode voting to register.");
                }

                Ok(result.to_string())
            }
            Err(e) => Err(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to check availability: {}", e),
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
