//! Cryptographic utilities for key validation

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::identity::KeyType;
use std::ffi::{c_char, CStr};

/// Validate that a private key corresponds to a public key using DPP's public_key_data_from_private_key_data
///
/// # Safety
/// - `private_key_hex` and `public_key_hex` must be valid, non-null pointers to NUL-terminated C strings that
///   remain valid for the duration of the call.
/// - `key_type` and `is_testnet` are passed by value; no references are retained.
/// - On success, the returned `DashSDKResult` contains a heap-allocated C string pointer which must be freed using
///   the SDK's free routine. It may also return no data (null pointer) to indicate success without payload.
/// - Passing invalid or dangling pointers results in undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_validate_private_key_for_public_key(
    private_key_hex: *const c_char,
    public_key_hex: *const c_char,
    key_type: u8,
    is_testnet: bool,
) -> DashSDKResult {
    if private_key_hex.is_null() || public_key_hex.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Private key or public key is null".to_string(),
        ));
    }

    let private_key_str = match CStr::from_ptr(private_key_hex).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid private key string: {}", e),
            ))
        }
    };

    let public_key_str = match CStr::from_ptr(public_key_hex).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid public key string: {}", e),
            ))
        }
    };

    // Decode private key hex
    let private_key_bytes = match hex::decode(private_key_str) {
        Ok(bytes) if bytes.len() == 32 => bytes,
        Ok(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Private key must be exactly 32 bytes".to_string(),
            ))
        }
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid private key hex: {}", e),
            ))
        }
    };

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&private_key_bytes);

    // Parse key type
    let key_type = match KeyType::try_from(key_type) {
        Ok(kt) => kt,
        Err(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid key type: {}", key_type),
            ))
        }
    };

    let network = if is_testnet {
        Network::Testnet
    } else {
        Network::Dash
    };

    // Use DPP's public_key_data_from_private_key_data to derive the public key
    let derived_public_key_data =
        match key_type.public_key_data_from_private_key_data(&key_array, network) {
            Ok(data) => data,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::CryptoError,
                    format!("Failed to derive public key: {}", e),
                ))
            }
        };

    // Decode the expected public key
    let expected_public_key_bytes = match hex::decode(public_key_str) {
        Ok(bytes) => bytes,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid public key hex: {}", e),
            ))
        }
    };

    // Compare
    let is_valid = derived_public_key_data == expected_public_key_bytes;

    // Return boolean as a string
    let result_str = if is_valid { "true" } else { "false" };
    match std::ffi::CString::new(result_str) {
        Ok(c_str) => DashSDKResult::success_string(c_str.into_raw()),
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("Failed to create result string: {}", e),
        )),
    }
}

/// Convert private key to WIF format
///
/// # Safety
/// - `private_key_hex` must be a valid, non-null pointer to a NUL-terminated C string representing a 32-byte hex key
///   and remain valid for the duration of the call.
/// - `is_testnet` is passed by value.
/// - On success, the returned `DashSDKResult` contains a heap-allocated C string pointer which must be freed using
///   the SDK's free routine.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_private_key_to_wif(
    private_key_hex: *const c_char,
    is_testnet: bool,
) -> DashSDKResult {
    if private_key_hex.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Private key is null".to_string(),
        ));
    }

    let private_key_str = match CStr::from_ptr(private_key_hex).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid private key string: {}", e),
            ))
        }
    };

    // Decode private key hex
    let private_key_bytes = match hex::decode(private_key_str) {
        Ok(bytes) if bytes.len() == 32 => bytes,
        Ok(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Private key must be exactly 32 bytes".to_string(),
            ))
        }
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid private key hex: {}", e),
            ))
        }
    };

    // Create PrivateKey from bytes
    let network = if is_testnet {
        Network::Testnet
    } else {
        Network::Dash
    };

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&private_key_bytes);
    match dash_sdk::dpp::dashcore::PrivateKey::from_byte_array(&key_array, network) {
        Ok(private_key) => {
            let wif = private_key.to_wif();
            match std::ffi::CString::new(wif) {
                Ok(c_str) => DashSDKResult::success_string(c_str.into_raw()),
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Failed to create result string: {}", e),
                )),
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::CryptoError,
            format!("Failed to create private key: {}", e),
        )),
    }
}

/// Get public key data from private key data
///
/// # Safety
/// - `private_key_hex` must be a valid, non-null pointer to a NUL-terminated C string representing a 32-byte hex key
///   and remain valid for the duration of the call.
/// - `key_type` and `is_testnet` are passed by value; no references are retained.
/// - On success, the returned `DashSDKResult` contains a heap-allocated C string pointer which must be freed using
///   the SDK's free routine.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_public_key_data_from_private_key_data(
    private_key_hex: *const c_char,
    key_type: u8,
    is_testnet: bool,
) -> DashSDKResult {
    if private_key_hex.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Private key is null".to_string(),
        ));
    }

    let private_key_str = match CStr::from_ptr(private_key_hex).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid private key string: {}", e),
            ))
        }
    };

    // Decode private key hex
    let private_key_bytes = match hex::decode(private_key_str) {
        Ok(bytes) if bytes.len() == 32 => bytes,
        Ok(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Private key must be exactly 32 bytes".to_string(),
            ))
        }
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid private key hex: {}", e),
            ))
        }
    };

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&private_key_bytes);

    // Parse key type
    let key_type = match KeyType::try_from(key_type) {
        Ok(kt) => kt,
        Err(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid key type: {}", key_type),
            ))
        }
    };

    let network = if is_testnet {
        Network::Testnet
    } else {
        Network::Dash
    };

    // Use DPP's public_key_data_from_private_key_data to derive the public key
    match key_type.public_key_data_from_private_key_data(&key_array, network) {
        Ok(data) => {
            let hex_string = hex::encode(&data);
            match std::ffi::CString::new(hex_string) {
                Ok(c_str) => DashSDKResult::success_string(c_str.into_raw()),
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Failed to create result string: {}", e),
                )),
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::CryptoError,
            format!("Failed to derive public key: {}", e),
        )),
    }
}
