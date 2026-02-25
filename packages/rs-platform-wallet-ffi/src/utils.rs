use crate::error::*;
use std::os::raw::{c_char, c_uchar};

/// Serialize any object to JSON bytes
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_serialize_to_json_bytes(
    json_string: *const c_char,
    out_bytes: *mut *mut c_uchar,
    out_len: *mut usize,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if json_string.is_null() || out_bytes.is_null() || out_len.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let json_str = unsafe {
        match std::ffi::CStr::from_ptr(json_string).to_str() {
            Ok(s) => s,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        "Invalid UTF-8 in JSON string",
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        }
    };

    let bytes = json_str.as_bytes().to_vec();
    let len = bytes.len();
    let ptr = bytes.as_ptr() as *mut c_uchar;
    std::mem::forget(bytes);

    unsafe {
        *out_bytes = ptr;
        *out_len = len;
    }

    PlatformWalletFFIResult::Success
}

/// Deserialize JSON bytes to string
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_deserialize_from_json_bytes(
    bytes: *const c_uchar,
    len: usize,
    out_json_string: *mut *mut c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if bytes.is_null() || out_json_string.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let data = unsafe { std::slice::from_raw_parts(bytes, len) };

    match std::str::from_utf8(data) {
        Ok(s) => match std::ffi::CString::new(s) {
            Ok(c_str) => {
                unsafe { *out_json_string = c_str.into_raw() };
                PlatformWalletFFIResult::Success
            }
            Err(_) => {
                if !out_error.is_null() {
                    unsafe {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorDeserialization,
                            "Failed to convert to C string",
                        );
                    }
                }
                PlatformWalletFFIResult::ErrorDeserialization
            }
        },
        Err(_) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        "Invalid UTF-8 in bytes",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorUtf8Conversion
        }
    }
}

/// Free bytes allocated by FFI functions
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_bytes_free(bytes: *mut c_uchar, len: usize) {
    if !bytes.is_null() && len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(bytes, len, len);
        }
    }
}

/// Generate random identifier
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_generate_random_identifier(
    out_id: *mut crate::types::IdentifierBytes,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_id.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let id = dpp::prelude::Identifier::random();
    unsafe { *out_id = id.into() };

    PlatformWalletFFIResult::Success
}

/// Convert identifier to hex string
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identifier_to_hex(
    id: crate::types::IdentifierBytes,
    out_hex: *mut *mut c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_hex.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let identifier = match id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid identifier: {}", e),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    let hex = identifier.to_string(dpp::platform_value::string_encoding::Encoding::Base58);
    match std::ffi::CString::new(hex) {
        Ok(c_str) => {
            unsafe { *out_hex = c_str.into_raw() };
            PlatformWalletFFIResult::Success
        }
        Err(_) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorSerialization,
                        "Failed to convert hex to C string",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorSerialization
        }
    }
}

/// Convert hex string to identifier
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identifier_from_hex(
    hex: *const c_char,
    out_id: *mut crate::types::IdentifierBytes,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if hex.is_null() || out_id.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let hex_str = unsafe {
        match std::ffi::CStr::from_ptr(hex).to_str() {
            Ok(s) => s,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        "Invalid UTF-8 in hex string",
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        }
    };

    match dpp::prelude::Identifier::from_string(
        hex_str,
        dpp::platform_value::string_encoding::Encoding::Base58,
    ) {
        Ok(identifier) => {
            unsafe { *out_id = identifier.into() };
            PlatformWalletFFIResult::Success
        }
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Failed to parse identifier: {}", e),
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidIdentifier
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_json_bytes() {
        unsafe {
            let json = std::ffi::CString::new(r#"{"test":"value"}"#).unwrap();
            let mut bytes: *mut c_uchar = std::ptr::null_mut();
            let mut len: usize = 0;
            let mut error = PlatformWalletFFIError::success();

            // Serialize
            let result = platform_wallet_serialize_to_json_bytes(
                json.as_ptr(),
                &mut bytes,
                &mut len,
                &mut error,
            );
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert!(!bytes.is_null());
            assert!(len > 0);

            // Deserialize
            let mut json_out: *mut c_char = std::ptr::null_mut();
            let result =
                platform_wallet_deserialize_from_json_bytes(bytes, len, &mut json_out, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert!(!json_out.is_null());

            let json_str = std::ffi::CStr::from_ptr(json_out).to_str().unwrap();
            assert_eq!(json_str, r#"{"test":"value"}"#);

            // Cleanup
            platform_wallet_bytes_free(bytes, len);
            crate::platform_wallet_string_free(json_out);
        }
    }

    #[test]
    fn test_generate_random_identifier() {
        unsafe {
            let mut id = crate::types::IdentifierBytes { bytes: [0u8; 32] };
            let mut error = PlatformWalletFFIError::success();

            let result = platform_wallet_generate_random_identifier(&mut id, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);

            // Check that it's not all zeros
            assert_ne!(id.bytes, [0u8; 32]);
        }
    }

    #[test]
    fn test_identifier_to_from_hex() {
        unsafe {
            let mut id = crate::types::IdentifierBytes { bytes: [0u8; 32] };
            let mut error = PlatformWalletFFIError::success();

            // Generate random ID
            platform_wallet_generate_random_identifier(&mut id, &mut error);

            // Convert to hex
            let mut hex: *mut c_char = std::ptr::null_mut();
            let result = platform_wallet_identifier_to_hex(id, &mut hex, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert!(!hex.is_null());

            // Convert back from hex
            let mut id2 = crate::types::IdentifierBytes { bytes: [0u8; 32] };
            let result = platform_wallet_identifier_from_hex(hex, &mut id2, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);

            // Should match
            assert_eq!(id.bytes, id2.bytes);

            // Cleanup
            crate::platform_wallet_string_free(hex);
        }
    }
}
