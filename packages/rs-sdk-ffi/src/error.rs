//! Error handling for FFI layer

use std::ffi::{CString, NulError};
use std::os::raw::c_char;
use thiserror::Error;

/// Error codes returned by FFI functions
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKErrorCode {
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
pub struct DashSDKError {
    /// Error code
    pub code: DashSDKErrorCode,
    /// Human-readable error message (null-terminated C string)
    /// Caller must free this with dash_sdk_error_free
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

impl DashSDKError {
    /// Create a new error
    pub fn new(code: DashSDKErrorCode, message: String) -> Self {
        let c_message = CString::new(message)
            .unwrap_or_else(|_| CString::new("Error message contains null byte").unwrap());

        DashSDKError {
            code,
            message: c_message.into_raw(),
        }
    }

    /// Create a success result
    pub fn success() -> Self {
        DashSDKError {
            code: DashSDKErrorCode::Success,
            message: std::ptr::null_mut(),
        }
    }
}

impl From<FFIError> for DashSDKError {
    fn from(err: FFIError) -> Self {
        let (code, message) = match &err {
            FFIError::InvalidParameter(_) => (DashSDKErrorCode::InvalidParameter, err.to_string()),
            FFIError::SDKError(sdk_err) => {
                // Extract more detailed error information
                let error_str = sdk_err.to_string();

                // Try to determine error type from the message
                let (code, detailed_msg) = if error_str.contains("timeout")
                    || error_str.contains("Timeout")
                {
                    (DashSDKErrorCode::Timeout, error_str)
                } else if error_str.contains("I/O error") || error_str.contains("connection") {
                    (
                        DashSDKErrorCode::NetworkError,
                        format!("Network connection failed: {}", error_str),
                    )
                } else if error_str.contains("DAPI") || error_str.contains("dapi") {
                    // Check for specific DAPI issues
                    if error_str.contains("No available addresses")
                        || error_str.contains("empty address list")
                    {
                        (DashSDKErrorCode::NetworkError,
                         "Cannot connect to network: No DAPI addresses configured. The SDK needs masternode quorum information to connect to the network.".to_string())
                    } else {
                        (
                            DashSDKErrorCode::NetworkError,
                            format!("DAPI error: {}", error_str),
                        )
                    }
                } else if error_str.contains("protocol") || error_str.contains("Protocol") {
                    (DashSDKErrorCode::ProtocolError, error_str)
                } else if error_str.contains("not found") || error_str.contains("Not found") {
                    (DashSDKErrorCode::NotFound, error_str)
                } else {
                    // Default to network error with the original message
                    (
                        DashSDKErrorCode::NetworkError,
                        format!("Failed to fetch balances: {}", error_str),
                    )
                };

                (code, detailed_msg)
            }
            FFIError::SerializationError(_) => {
                (DashSDKErrorCode::SerializationError, err.to_string())
            }
            FFIError::Utf8Error(_) => (DashSDKErrorCode::InvalidParameter, err.to_string()),
            FFIError::NullPointer => (
                DashSDKErrorCode::InvalidParameter,
                "Null pointer".to_string(),
            ),
            FFIError::InternalError(_) => (DashSDKErrorCode::InternalError, err.to_string()),
            FFIError::NotImplemented(_) => (DashSDKErrorCode::NotImplemented, err.to_string()),
            FFIError::InvalidState(_) => (DashSDKErrorCode::InvalidState, err.to_string()),
            FFIError::NotFound(_) => (DashSDKErrorCode::NotFound, err.to_string()),
            FFIError::NulError(_) => (DashSDKErrorCode::InvalidParameter, err.to_string()),
        };

        DashSDKError::new(code, message)
    }
}

/// Free an error message
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_error_free(error: *mut DashSDKError) {
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
                let error: $crate::DashSDKError = e.into();
                return Box::into_raw(Box::new(error));
            }
        }
    };
}
