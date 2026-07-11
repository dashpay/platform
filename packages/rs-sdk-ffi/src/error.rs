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
    /// Drive returned an internal error (e.g., storage-level constraint violation)
    DriveInternalError = 10,
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

                // Match typed enum variants first — string matching can collide with
                // substrings inside Drive messages (e.g., "data contract not found"
                // emitted as a DriveInternalError would otherwise be misclassified
                // as NotFound).
                let (code, detailed_msg) = if let dash_sdk::Error::DriveInternalError(inner) =
                    sdk_err
                {
                    // The DriveInternalError code already conveys the classification;
                    // emit only the inner Drive message so downstream FFI consumers
                    // don't double-render the "Drive internal error: " prefix.
                    (DashSDKErrorCode::DriveInternalError, inner.clone())
                } else if matches!(
                    sdk_err,
                    dash_sdk::Error::DapiClientError(_)
                        | dash_sdk::Error::NoAvailableAddressesToRetry(_)
                ) {
                    // Transport / connectivity failure (e.g. all DAPI nodes
                    // unreachable or serving expired TLS certificates). Match the
                    // typed variant rather than the Display string: the message
                    // ("Dapi client error: transport error: ...") matches none of
                    // the substrings below, so it would otherwise fall through to
                    // InternalError and surface in the UI as a misleading
                    // "Internal Error" for what is really a network problem.
                    (DashSDKErrorCode::NetworkError, error_str)
                } else if matches!(sdk_err, dash_sdk::Error::TimeoutReached(_, _))
                    || error_str.contains("timeout")
                    || error_str.contains("Timeout")
                {
                    // Typed SDK timeout, plus a substring fallback for timeouts
                    // surfaced inside other error types' Display strings.
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
                    // Unclassified SDK error: pass the original message through
                    // unchanged and map to InternalError rather than guessing a
                    // network cause. (Previously this hardcoded a "Failed to fetch
                    // balances:" prefix and the NetworkError code, mislabeling
                    // unrelated failures such as proof-verification errors from
                    // getDataContractHistory.)
                    (DashSDKErrorCode::InternalError, error_str)
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
///
/// # Safety
/// - `error` must be a pointer previously returned by this SDK or null (no-op).
/// - After this call, `error` becomes invalid and must not be used again.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn classify(err: dash_sdk::Error) -> DashSDKErrorCode {
        let dash_sdk_error: DashSDKError = FFIError::SDKError(err).into();
        let code = dash_sdk_error.code;
        // Free the message we allocated via DashSDKError::new.
        unsafe {
            if !dash_sdk_error.message.is_null() {
                let _ = CString::from_raw(dash_sdk_error.message);
            }
        }
        code
    }

    #[test]
    fn drive_internal_error_with_not_found_substring_maps_to_drive_internal_error() {
        // Drive emits messages like "data contract not found"; the Display form is
        // "Drive internal error: data contract not found …". Typed-variant matching
        // must take precedence over substring heuristics.
        let err = dash_sdk::Error::DriveInternalError("data contract not found 0x123".to_string());
        assert_eq!(classify(err), DashSDKErrorCode::DriveInternalError);
    }

    #[test]
    fn drive_internal_error_plain_maps_to_drive_internal_error() {
        let err = dash_sdk::Error::DriveInternalError("storage layer constraint".to_string());
        assert_eq!(classify(err), DashSDKErrorCode::DriveInternalError);
    }

    #[test]
    fn drive_internal_error_message_omits_redundant_variant_prefix() {
        let err = dash_sdk::Error::DriveInternalError("storage layer constraint".to_string());
        let dash_sdk_error: DashSDKError = FFIError::SDKError(err).into();
        let message = unsafe {
            let m = std::ffi::CStr::from_ptr(dash_sdk_error.message)
                .to_string_lossy()
                .into_owned();
            let _ = CString::from_raw(dash_sdk_error.message);
            m
        };
        assert_eq!(dash_sdk_error.code, DashSDKErrorCode::DriveInternalError);
        assert_eq!(message, "storage layer constraint");
    }

    #[test]
    fn generic_not_found_still_maps_to_not_found() {
        let err = dash_sdk::Error::Generic("identity not found".to_string());
        assert_eq!(classify(err), DashSDKErrorCode::NotFound);
    }

    #[test]
    fn dapi_client_error_maps_to_network_error() {
        // The Display form is "Dapi client error: …", which matches none of the
        // substring heuristics ("DAPI"/"dapi"/"connection"/…). It must be
        // classified as NetworkError via the typed variant so a transient
        // transport failure (e.g. an evonode serving an expired TLS cert) does
        // not surface in the UI as a misleading "Internal Error".
        let err = dash_sdk::Error::DapiClientError(
            dash_sdk::dapi_client::DapiClientError::NoAvailableAddresses,
        );
        assert_eq!(classify(err), DashSDKErrorCode::NetworkError);
    }

    #[test]
    fn timeout_reached_maps_to_timeout() {
        let err = dash_sdk::Error::TimeoutReached(
            std::time::Duration::from_secs(8),
            "fetch protocol version upgrade state".to_string(),
        );
        assert_eq!(classify(err), DashSDKErrorCode::Timeout);
    }

    #[test]
    fn unclassified_error_maps_to_internal_error_without_balance_prefix() {
        // A proof-verification failure (e.g. from getDataContractHistory) matches
        // none of the substring heuristics and must fall through the catch-all.
        // It should be classified as InternalError and keep its original Display
        // verbatim — no copy-pasted "Failed to fetch balances:" prefix.
        let err = dash_sdk::Error::Generic(
            "Proof verification error: corrupted element for the historical contract".to_string(),
        );
        // The catch-all passes the SDK error's Display through unchanged.
        let expected = err.to_string();

        let dash_sdk_error: DashSDKError = FFIError::SDKError(err).into();
        let rendered = unsafe {
            let m = std::ffi::CStr::from_ptr(dash_sdk_error.message)
                .to_string_lossy()
                .into_owned();
            let _ = CString::from_raw(dash_sdk_error.message);
            m
        };

        assert_eq!(dash_sdk_error.code, DashSDKErrorCode::InternalError);
        assert_eq!(rendered, expected);
        assert!(!rendered.contains("Failed to fetch balances"));
    }
}
