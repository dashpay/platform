//! Cryptographic utilities for key validation

use crate::types::{FFINetwork, Network};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::dashcore::PrivateKey;
use dash_sdk::dpp::identity::KeyType;
use dash_sdk::dpp::secp256k1::SecretKey;
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

/// Reads a secp256k1 private key typed by a person, as 64 hex characters or in WIF, and
/// writes its 32 secret bytes to `out_private_key`.
///
/// Leading and trailing whitespace is ignored. Exactly 64 characters are read as hex;
/// anything else is read as WIF, whatever network the WIF names (the secret is the same on
/// every network). A WIF must mark the key compressed: Platform keys are formed from the
/// compressed public key, so an uncompressed WIF names a different key and is refused.
///
/// # Parameters
/// * `text` - The key as typed, a NUL-terminated string.
/// * `out_private_key` - Pointer to 32 writable bytes that receive the secret. They need not
///   be initialized.
///
/// # Returns
/// No data on success, with the secret written. `InvalidParameter` for a null pointer, text
/// that is neither form, an uncompressed WIF, or a value that is not a secp256k1 secret key
/// (zero, or not below the curve order); nothing is written then.
///
/// # Safety
/// `text` must be null or a valid NUL-terminated string, and `out_private_key` null or point
/// to 32 writable bytes, for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_private_key_bytes_from_text(
    text: *const c_char,
    out_private_key: *mut u8,
) -> DashSDKResult {
    let invalid = |message: String| {
        DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            message,
        ))
    };
    if text.is_null() || out_private_key.is_null() {
        return invalid("text or out_private_key is null".to_string());
    }
    let text = match CStr::from_ptr(text).to_str() {
        Ok(text) => text.trim(),
        Err(e) => return invalid(format!("Invalid private key string: {}", e)),
    };

    let mut secret = Zeroizing::new([0u8; 32]);
    if text.len() == 64 {
        let bytes = match hex::decode(text) {
            Ok(bytes) => Zeroizing::new(bytes),
            Err(e) => return invalid(format!("Invalid private key hex: {}", e)),
        };
        secret.copy_from_slice(&bytes);
        if let Err(e) = SecretKey::from_secret_bytes(*secret) {
            return invalid(format!("Invalid private key: {}", e));
        }
    } else {
        match PrivateKey::from_wif(text) {
            Ok(private_key) if !private_key.compressed => {
                return invalid(
                    "uncompressed WIF keys are not supported; Platform keys use the compressed \
                     public key"
                        .to_string(),
                )
            }
            Ok(private_key) => secret = private_key.inner.to_bytes(),
            Err(e) => {
                return invalid(format!(
                    "The private key is neither 64 hex characters nor WIF: {}",
                    e
                ))
            }
        }
    }

    // A raw copy: the caller's buffer may be uninitialized, so no `&mut [u8]` is formed over it.
    std::ptr::copy_nonoverlapping(secret.as_ptr(), out_private_key, 32);
    DashSDKResult::success(std::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_sdk_error_free;
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    /// The secret written for `text`, or the error code; the error is freed.
    fn read(text: &str) -> Result<[u8; 32], DashSDKErrorCode> {
        let text = CString::new(text).expect("no NUL");
        let mut out = [0xEEu8; 32];
        unsafe {
            let result = dash_sdk_private_key_bytes_from_text(text.as_ptr(), out.as_mut_ptr());
            assert!(result.data.is_null());
            if result.error.is_null() {
                return Ok(out);
            }
            let code = (*result.error).code;
            dash_sdk_error_free(result.error);
            assert_eq!(out, [0xEEu8; 32], "nothing is written on a refusal");
            Err(code)
        }
    }

    #[test]
    fn should_read_a_hex_key_and_the_same_key_in_wif() {
        let secret = [2u8; 32];
        let wif = PrivateKey::from_byte_array(&secret, Network::Testnet)
            .expect("a secret key")
            .to_wif();
        assert_eq!(read(&hex::encode(secret)), Ok(secret));
        assert_eq!(read(&format!("  {}\n", hex::encode(secret))), Ok(secret));
        assert_eq!(read(&wif), Ok(secret));
    }

    #[test]
    fn should_refuse_an_uncompressed_wif() {
        let secret = [2u8; 32];
        let mut private_key =
            PrivateKey::from_byte_array(&secret, Network::Testnet).expect("a secret key");
        private_key.compressed = false;
        let wif = private_key.to_wif();
        assert_eq!(read(&wif), Err(DashSDKErrorCode::InvalidParameter));
    }

    #[test]
    fn should_write_the_secret_into_an_uninitialized_buffer() {
        let secret = [2u8; 32];
        let text = CString::new(hex::encode(secret)).expect("no NUL");
        let mut out = MaybeUninit::<[u8; 32]>::uninit();
        let out = unsafe {
            let result =
                dash_sdk_private_key_bytes_from_text(text.as_ptr(), out.as_mut_ptr().cast::<u8>());
            assert!(result.error.is_null());
            assert!(result.data.is_null());
            out.assume_init()
        };
        assert_eq!(out, secret);
    }

    #[test]
    fn should_refuse_text_that_is_not_a_private_key() {
        for text in [
            "",
            &"0".repeat(64),
            &"zz".repeat(32),
            &"ab".repeat(31),
            "cNotAWifKey",
        ] {
            assert_eq!(
                read(text),
                Err(DashSDKErrorCode::InvalidParameter),
                "{text}"
            );
        }
        unsafe {
            let mut out = [0u8; 32];
            let result = dash_sdk_private_key_bytes_from_text(std::ptr::null(), out.as_mut_ptr());
            assert!(!result.error.is_null());
            dash_sdk_error_free(result.error);
            let text = CString::new(hex::encode([2u8; 32])).expect("no NUL");
            let result = dash_sdk_private_key_bytes_from_text(text.as_ptr(), std::ptr::null_mut());
            assert!(!result.error.is_null());
            dash_sdk_error_free(result.error);
        }
    }
}
