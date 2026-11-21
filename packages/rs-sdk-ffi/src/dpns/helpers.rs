//! DPNS helper functions for validation and normalization

use crate::{utils, DashSDKError, DashSDKErrorCode, DashSDKResult};
use std::ffi::CStr;

/// Convert a string to homograph-safe characters by replacing 'o', 'i', and 'l'
/// with '0', '1', and '1' respectively to prevent homograph attacks
///
/// # Safety
/// - `name` must be a valid null-terminated C string
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_normalize_username(
    name: *const std::os::raw::c_char,
) -> DashSDKResult {
    if name.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Name is null".to_string(),
        ));
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid UTF-8 string: {}", e),
            ));
        }
    };

    let normalized = dash_sdk::platform::dpns_usernames::convert_to_homograph_safe_chars(name_str);

    match utils::c_string_from(normalized) {
        Ok(c_string) => DashSDKResult::success(c_string as *mut std::os::raw::c_void),
        Err(e) => DashSDKResult::error(e),
    }
}

/// Check if a username is valid according to DPNS rules
///
/// A username is valid if:
/// - It's between 3 and 63 characters long
/// - It starts and ends with alphanumeric characters (a-zA-Z0-9)
/// - It contains only alphanumeric characters and hyphens
/// - It doesn't have consecutive hyphens
///
/// # Safety
/// - `name` must be a valid null-terminated C string
///
/// # Returns
/// - 1 if the username is valid
/// - 0 if the username is invalid
/// - -1 if there's an error
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_is_valid_username(name: *const std::os::raw::c_char) -> i32 {
    if name.is_null() {
        return -1;
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    if dash_sdk::platform::dpns_usernames::is_valid_username(name_str) {
        1
    } else {
        0
    }
}

/// Check if a username is contested (requires masternode voting)
///
/// A username is contested if its normalized label:
/// - Is between 3 and 19 characters long (inclusive)
/// - Contains only lowercase letters a-z, digits 0-1, and hyphens
///
/// # Safety
/// - `name` must be a valid null-terminated C string
///
/// # Returns
/// - 1 if the username is contested
/// - 0 if the username is not contested
/// - -1 if there's an error
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_is_contested_username(
    name: *const std::os::raw::c_char,
) -> i32 {
    if name.is_null() {
        return -1;
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    if dash_sdk::platform::dpns_usernames::is_contested_username(name_str) {
        1
    } else {
        0
    }
}

/// Get a validation message for a username
///
/// Returns a descriptive message about why a username is invalid, or "valid" if it's valid.
///
/// # Safety
/// - `name` must be a valid null-terminated C string
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_get_validation_message(
    name: *const std::os::raw::c_char,
) -> DashSDKResult {
    if name.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Name is null".to_string(),
        ));
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid UTF-8 string: {}", e),
            ));
        }
    };

    let message = if name_str.len() < 3 {
        "Name must be at least 3 characters long"
    } else if name_str.len() > 63 {
        "Name must be 63 characters or less"
    } else if !name_str
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        "Name must start with an alphanumeric character"
    } else if !name_str
        .chars()
        .last()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        "Name must end with an alphanumeric character"
    } else if name_str.contains("--") {
        "Name cannot contain consecutive hyphens"
    } else if !name_str
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        "Name can only contain letters, numbers, and hyphens"
    } else if dash_sdk::platform::dpns_usernames::is_valid_username(name_str) {
        "valid"
    } else {
        "Invalid username"
    };

    match utils::c_string_from(message.to_string()) {
        Ok(c_string) => DashSDKResult::success(c_string as *mut std::os::raw::c_void),
        Err(e) => DashSDKResult::error(e),
    }
}
