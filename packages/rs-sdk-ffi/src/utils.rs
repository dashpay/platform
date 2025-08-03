//! Utility functions for the FFI

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};

/// Convert a hex string to base58
/// 
/// # Parameters
/// - `hex_string`: Hex encoded string (must be 64 characters for identity IDs)
/// 
/// # Returns
/// - Base58 encoded string on success
/// - Error if the hex string is invalid
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_utils_hex_to_base58(
    hex_string: *const c_char,
) -> DashSDKResult {
    if hex_string.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Hex string is null".to_string(),
        ));
    }

    let hex_str = match CStr::from_ptr(hex_string).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid UTF-8 string: {}", e),
            ))
        }
    };

    // Try to parse as hex and convert to base58
    match hex::decode(hex_str) {
        Ok(bytes) => {
            // For identity IDs, we expect exactly 32 bytes
            if bytes.len() == 32 {
                match Identifier::from_bytes(&bytes) {
                    Ok(id) => {
                        let base58 = id.to_string(Encoding::Base58);
                        match CString::new(base58) {
                            Ok(c_str) => {
                                DashSDKResult::success(Box::into_raw(c_str.into_boxed_c_str()) as *mut std::os::raw::c_void)
                            }
                            Err(e) => {
                                DashSDKResult::error(DashSDKError::new(
                                    DashSDKErrorCode::InternalError,
                                    format!("Failed to create C string: {}", e),
                                ))
                            }
                        }
                    }
                    Err(e) => {
                        DashSDKResult::error(DashSDKError::new(
                            DashSDKErrorCode::InvalidParameter,
                            format!("Invalid identifier bytes: {}", e),
                        ))
                    }
                }
            } else {
                DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Expected 32 bytes for identity ID, got {}", bytes.len()),
                ))
            }
        }
        Err(e) => {
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid hex string: {}", e),
            ))
        }
    }
}

/// Convert a base58 string to hex
/// 
/// # Parameters
/// - `base58_string`: Base58 encoded string
/// 
/// # Returns
/// - Hex encoded string on success
/// - Error if the base58 string is invalid
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_utils_base58_to_hex(
    base58_string: *const c_char,
) -> DashSDKResult {
    if base58_string.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Base58 string is null".to_string(),
        ));
    }

    let base58_str = match CStr::from_ptr(base58_string).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid UTF-8 string: {}", e),
            ))
        }
    };

    // Try to parse as base58 identifier
    match Identifier::from_string(base58_str, Encoding::Base58) {
        Ok(id) => {
            let hex = hex::encode(id.to_buffer());
            match CString::new(hex) {
                Ok(c_str) => {
                    DashSDKResult::success(Box::into_raw(c_str.into_boxed_c_str()) as *mut std::os::raw::c_void)
                }
                Err(e) => {
                    DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InternalError,
                        format!("Failed to create C string: {}", e),
                    ))
                }
            }
        }
        Err(e) => {
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid base58 string: {}", e),
            ))
        }
    }
}

/// Validate if a string is valid base58
/// 
/// # Parameters
/// - `string`: String to validate
/// 
/// # Returns
/// - 1 if valid base58, 0 if invalid
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_utils_is_valid_base58(
    string: *const c_char,
) -> u8 {
    if string.is_null() {
        return 0;
    }

    let str = match CStr::from_ptr(string).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    // Check if it can be decoded as base58
    match Identifier::from_string(str, Encoding::Base58) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}