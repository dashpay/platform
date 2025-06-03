use std::ffi::CString;
use std::os::raw::c_char;

/// Error codes for Swift Dash Platform operations
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwiftDashErrorCode {
    /// Operation completed successfully
    Success = 0,
    /// Invalid parameter passed to function
    InvalidParameter = 1,
    /// SDK not initialized or in invalid state
    InvalidState = 2,
    /// Network error occurred
    NetworkError = 3,
    /// Serialization/deserialization error
    SerializationError = 4,
    /// Platform protocol error
    ProtocolError = 5,
    /// Cryptographic operation failed
    CryptoError = 6,
    /// Resource not found
    NotFound = 7,
    /// Operation timed out
    Timeout = 8,
    /// Feature not implemented
    NotImplemented = 9,
    /// Internal error
    InternalError = 99,
}

/// Error structure for Swift interop
#[repr(C)]
pub struct SwiftDashError {
    /// Error code
    pub code: SwiftDashErrorCode,
    /// Human-readable error message (null-terminated C string)
    /// Caller must free this with swift_dash_error_free
    pub message: *mut c_char,
}

impl SwiftDashError {
    /// Create a new error
    pub fn new(code: SwiftDashErrorCode, message: String) -> Self {
        let c_message = CString::new(message)
            .unwrap_or_else(|_| CString::new("Error message contains null byte").unwrap());

        SwiftDashError {
            code,
            message: c_message.into_raw(),
        }
    }

    /// Create a success result
    pub fn success() -> Self {
        SwiftDashError {
            code: SwiftDashErrorCode::Success,
            message: std::ptr::null_mut(),
        }
    }

    pub fn invalid_parameter(message: &str) -> Self {
        Self::new(
            SwiftDashErrorCode::InvalidParameter,
            format!("Invalid parameter: {}", message),
        )
    }

    pub fn invalid_state(message: &str) -> Self {
        Self::new(
            SwiftDashErrorCode::InvalidState,
            format!("Invalid state: {}", message),
        )
    }

    pub fn network_error(message: &str) -> Self {
        Self::new(
            SwiftDashErrorCode::NetworkError,
            format!("Network error: {}", message),
        )
    }

    pub fn not_found(message: &str) -> Self {
        Self::new(
            SwiftDashErrorCode::NotFound,
            format!("Not found: {}", message),
        )
    }

    pub fn internal_error(message: &str) -> Self {
        Self::new(
            SwiftDashErrorCode::InternalError,
            format!("Internal error: {}", message),
        )
    }
}

impl From<ios_sdk_ffi::IOSSDKError> for SwiftDashError {
    fn from(error: ios_sdk_ffi::IOSSDKError) -> Self {
        let message = if error.message.is_null() {
            "Unknown error".to_string()
        } else {
            unsafe {
                std::ffi::CStr::from_ptr(error.message)
                    .to_string_lossy()
                    .to_string()
            }
        };

        let code = match error.code {
            ios_sdk_ffi::IOSSDKErrorCode::Success => SwiftDashErrorCode::Success,
            ios_sdk_ffi::IOSSDKErrorCode::InvalidParameter => SwiftDashErrorCode::InvalidParameter,
            ios_sdk_ffi::IOSSDKErrorCode::InvalidState => SwiftDashErrorCode::InvalidState,
            ios_sdk_ffi::IOSSDKErrorCode::NetworkError => SwiftDashErrorCode::NetworkError,
            ios_sdk_ffi::IOSSDKErrorCode::SerializationError => {
                SwiftDashErrorCode::SerializationError
            }
            ios_sdk_ffi::IOSSDKErrorCode::ProtocolError => SwiftDashErrorCode::ProtocolError,
            ios_sdk_ffi::IOSSDKErrorCode::CryptoError => SwiftDashErrorCode::CryptoError,
            ios_sdk_ffi::IOSSDKErrorCode::NotFound => SwiftDashErrorCode::NotFound,
            ios_sdk_ffi::IOSSDKErrorCode::Timeout => SwiftDashErrorCode::Timeout,
            ios_sdk_ffi::IOSSDKErrorCode::NotImplemented => SwiftDashErrorCode::NotImplemented,
            ios_sdk_ffi::IOSSDKErrorCode::InternalError => SwiftDashErrorCode::InternalError,
        };

        Self::new(code, message)
    }
}

/// Swift result that wraps either success or error
#[repr(C)]
pub struct SwiftDashResult {
    pub success: bool,
    pub data: *mut std::os::raw::c_void,
    pub error: *mut SwiftDashError,
}

impl SwiftDashResult {
    pub fn success_with_data(data: *mut std::os::raw::c_void) -> Self {
        SwiftDashResult {
            success: true,
            data,
            error: std::ptr::null_mut(),
        }
    }

    pub fn success() -> Self {
        SwiftDashResult {
            success: true,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        }
    }

    pub fn error(error: SwiftDashError) -> Self {
        SwiftDashResult {
            success: false,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(error)),
        }
    }

    pub fn from_ffi_result(ffi_result: ios_sdk_ffi::IOSSDKResult) -> Self {
        if ffi_result.error.is_null() {
            SwiftDashResult::success_with_data(ffi_result.data)
        } else {
            let error = unsafe { *Box::from_raw(ffi_result.error) };
            SwiftDashResult::error(SwiftDashError::from(error))
        }
    }
}

/// Free an error message
#[no_mangle]
pub unsafe extern "C" fn swift_dash_error_free(error: *mut SwiftDashError) {
    if error.is_null() {
        return;
    }

    let error = Box::from_raw(error);
    if !error.message.is_null() {
        let _ = CString::from_raw(error.message);
    }
}

/// Free a C string allocated by Swift SDK
#[no_mangle]
pub unsafe extern "C" fn swift_dash_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    let _ = CString::from_raw(s);
}

/// Free bytes allocated by callback functions
#[no_mangle]
pub unsafe extern "C" fn swift_dash_bytes_free(bytes: *mut u8, len: usize) {
    if bytes.is_null() || len == 0 {
        return;
    }
    let _ = Vec::from_raw_parts(bytes, len, len);
}
