//! Cryptographic utilities for key validation

use crate::types::{FFINetwork, Network};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::identity::KeyType;
use std::ffi::{c_char, CStr};
use zeroize::Zeroizing;

/// Validate that a private key corresponds to a public key using DPP's public_key_data_from_private_key_data
///
/// # Safety
/// - `private_key_hex` and `public_key_hex` must be valid, non-null pointers to NUL-terminated C strings that
///   remain valid for the duration of the call.
/// - `key_type` and `network` are passed by value; no references are retained.
/// - On success, the returned `DashSDKResult` contains a heap-allocated C string pointer which must be freed using
///   the SDK's free routine. It may also return no data (null pointer) to indicate success without payload.
/// - Passing invalid or dangling pointers results in undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_validate_private_key_for_public_key(
    private_key_hex: *const c_char,
    public_key_hex: *const c_char,
    key_type: u8,
    network: FFINetwork,
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

    let mut key_array = Zeroizing::new([0u8; 32]);
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

    let network: Network = network.into();

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

/// Write the size in bytes of a private key of `key_type` to `out_size`, as DPP's
/// `KeyType::private_key_size` defines it, so callers compare a key's length against DPP's size
/// instead of keeping their own.
///
/// # Returns
/// A result with no data on success. An `InvalidParameter` error when `out_size` is null or
/// `key_type` is not a known key type, and `out_size` is left untouched.
///
/// # Safety
/// - `out_size` must be null or a valid, aligned pointer to writable memory for one `usize`.
/// - `key_type` is passed by value; no references are retained.
/// - On error, the returned `DashSDKResult` holds a heap-allocated error which must be freed with
///   `dash_sdk_error_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_private_key_size_for_key_type(
    key_type: u8,
    out_size: *mut usize,
) -> DashSDKResult {
    if out_size.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Output size pointer is null".to_string(),
        ));
    }

    let key_type = match KeyType::try_from(key_type) {
        Ok(kt) => kt,
        Err(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid key type: {}", key_type),
            ))
        }
    };

    *out_size = key_type.private_key_size();
    DashSDKResult::success(std::ptr::null_mut())
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
    network: FFINetwork,
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

    let network: Network = network.into();

    let mut key_array = Zeroizing::new([0u8; 32]);
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
/// - `key_type` and `network` are passed by value; no references are retained.
/// - On success, the returned `DashSDKResult` contains a heap-allocated C string pointer which must be freed using
///   the SDK's free routine.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_public_key_data_from_private_key_data(
    private_key_hex: *const c_char,
    key_type: u8,
    network: FFINetwork,
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

    let mut key_array = Zeroizing::new([0u8; 32]);
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

    let network: Network = network.into();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_sdk_error_free;

    #[test]
    fn should_report_dpps_private_key_size_for_each_key_type() {
        for key_type in KeyType::all_key_types() {
            let mut size = 0usize;
            let result =
                unsafe { dash_sdk_private_key_size_for_key_type(key_type as u8, &mut size) };
            assert!(result.error.is_null(), "{key_type} was refused");
            assert!(result.data.is_null(), "{key_type} returned data");
            assert_eq!(size, key_type.private_key_size(), "{key_type}");
        }
    }

    #[test]
    fn should_refuse_an_unknown_key_type_and_leave_the_size_untouched() {
        for unknown in [6u8, 255] {
            let mut size = usize::MAX;
            let result = unsafe { dash_sdk_private_key_size_for_key_type(unknown, &mut size) };
            assert!(result.data.is_null(), "key type {unknown} returned data");
            assert!(!result.error.is_null(), "key type {unknown} was accepted");
            let code = unsafe { (*result.error).code };
            unsafe { dash_sdk_error_free(result.error) };
            assert_eq!(
                code,
                DashSDKErrorCode::InvalidParameter,
                "key type {unknown}"
            );
            assert_eq!(size, usize::MAX, "key type {unknown} wrote a size");
        }
    }

    #[test]
    fn should_refuse_a_null_size_pointer() {
        let result = unsafe {
            dash_sdk_private_key_size_for_key_type(
                KeyType::REDPALLAS_HECATE as u8,
                std::ptr::null_mut(),
            )
        };
        assert!(!result.error.is_null(), "a null size pointer was accepted");
        let code = unsafe { (*result.error).code };
        unsafe { dash_sdk_error_free(result.error) };
        assert_eq!(code, DashSDKErrorCode::InvalidParameter);
    }
}
