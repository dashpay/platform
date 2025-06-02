//! Error handling for FFI layer

use std::ffi::{CString, NulError};
use std::os::raw::c_char;
use thiserror::Error;

/// Error codes returned by FFI functions
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IOSSDKErrorCode {
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

/// Error structure returned by FFI functions
#[repr(C)]
pub struct IOSSDKError {
    /// Error code
    pub code: IOSSDKErrorCode,
    /// Human-readable error message (null-terminated C string)
    /// Caller must free this with ios_sdk_error_free
    pub message: *mut c_char,
}

/// Internal error type for FFI operations
#[derive(Debug, Error)]
pub enum FFIError {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("SDK error: {0}")]
    SDKError(#[from] dash_sdk::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid UTF-8 string")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Null pointer")]
    NullPointer,

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("String contains null byte")]
    NulError(#[from] NulError),
}

impl IOSSDKError {
    /// Create a new error
    pub fn new(code: IOSSDKErrorCode, message: String) -> Self {
        let c_message = CString::new(message)
            .unwrap_or_else(|_| CString::new("Error message contains null byte").unwrap());

        IOSSDKError {
            code,
            message: c_message.into_raw(),
        }
    }

    /// Create a success result
    pub fn success() -> Self {
        IOSSDKError {
            code: IOSSDKErrorCode::Success,
            message: std::ptr::null_mut(),
        }
    }
}

impl From<FFIError> for IOSSDKError {
    fn from(err: FFIError) -> Self {
        let (code, message) = match &err {
            FFIError::InvalidParameter(_) => (IOSSDKErrorCode::InvalidParameter, err.to_string()),
            FFIError::SDKError(_) => (IOSSDKErrorCode::ProtocolError, err.to_string()),
            FFIError::SerializationError(_) => {
                (IOSSDKErrorCode::SerializationError, err.to_string())
            }
            FFIError::Utf8Error(_) => (IOSSDKErrorCode::InvalidParameter, err.to_string()),
            FFIError::NullPointer => (
                IOSSDKErrorCode::InvalidParameter,
                "Null pointer".to_string(),
            ),
            FFIError::InternalError(_) => (IOSSDKErrorCode::InternalError, err.to_string()),
            FFIError::NotImplemented(_) => (IOSSDKErrorCode::NotImplemented, err.to_string()),
            FFIError::InvalidState(_) => (IOSSDKErrorCode::InvalidState, err.to_string()),
            FFIError::NotFound(_) => (IOSSDKErrorCode::NotFound, err.to_string()),
            FFIError::NulError(_) => (IOSSDKErrorCode::InvalidParameter, err.to_string()),
        };

        IOSSDKError::new(code, message)
    }
}

/// Free an error message
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_error_free(error: *mut IOSSDKError) {
    if error.is_null() {
        return;
    }

    let error = Box::from_raw(error);
    if !error.message.is_null() {
        let _ = CString::from_raw(error.message);
    }
}

/// Helper macro for FFI error handling
#[macro_export]
macro_rules! ffi_result {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                let error: $crate::IOSSDKError = e.into();
                return Box::into_raw(Box::new(error));
            }
        }
    };
}
