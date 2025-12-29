use std::ffi::CString;
use std::os::raw::c_char;

/// FFI Result type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformWalletFFIResult {
    Success = 0,
    ErrorInvalidHandle = 1,
    ErrorInvalidParameter = 2,
    ErrorNullPointer = 3,
    ErrorSerialization = 4,
    ErrorDeserialization = 5,
    ErrorWalletOperation = 6,
    ErrorIdentityNotFound = 7,
    ErrorContactNotFound = 8,
    ErrorInvalidNetwork = 9,
    ErrorInvalidIdentifier = 10,
    ErrorMemoryAllocation = 11,
    ErrorUtf8Conversion = 12,
    ErrorUnknown = 99,
}

/// Error information structure
#[repr(C)]
pub struct PlatformWalletFFIError {
    pub code: PlatformWalletFFIResult,
    pub message: *mut c_char,
}

impl PlatformWalletFFIError {
    pub fn new(code: PlatformWalletFFIResult, message: impl Into<String>) -> Self {
        let msg = message.into();
        let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Invalid UTF-8").unwrap());
        Self {
            code,
            message: c_msg.into_raw(),
        }
    }

    pub fn success() -> Self {
        Self {
            code: PlatformWalletFFIResult::Success,
            message: std::ptr::null_mut(),
        }
    }
}

/// Free error message
#[no_mangle]
pub extern "C" fn platform_wallet_ffi_error_free(error: PlatformWalletFFIError) {
    if !error.message.is_null() {
        unsafe {
            let _ = CString::from_raw(error.message);
        }
    }
}

/// Convert Rust error to FFI error
pub trait ToFFIError {
    fn to_ffi_error(&self) -> PlatformWalletFFIError;
}

impl<E: std::error::Error> ToFFIError for E {
    fn to_ffi_error(&self) -> PlatformWalletFFIError {
        PlatformWalletFFIError::new(PlatformWalletFFIResult::ErrorUnknown, self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error =
            PlatformWalletFFIError::new(PlatformWalletFFIResult::ErrorInvalidHandle, "Test error");
        assert_eq!(error.code, PlatformWalletFFIResult::ErrorInvalidHandle);
        assert!(!error.message.is_null());

        // Clean up
        platform_wallet_ffi_error_free(error);
    }

    #[test]
    fn test_success_error() {
        let error = PlatformWalletFFIError::success();
        assert_eq!(error.code, PlatformWalletFFIResult::Success);
        assert!(error.message.is_null());
    }

    #[test]
    fn test_error_free() {
        let error = PlatformWalletFFIError::new(PlatformWalletFFIResult::ErrorUnknown, "Test");
        platform_wallet_ffi_error_free(error);
        // Should not crash
    }
}
