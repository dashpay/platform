//! Error handling for FFI layer
//!
//! # ABI stability
//!
//! The public C ABI struct [`DashSDKError`] is intentionally frozen: it always
//! consists of a [`DashSDKErrorCode`] discriminant plus an owned, NUL-terminated
//! `message` pointer. Consumers built against older headers continue to work as
//! before — the readable scalar message remains the primary surface for protocol
//! consensus errors (singular: the error's own `Display`; plural: `;`-joined).
//!
//! Structured details about consensus errors are exposed through a *sidecar*
//! lookup keyed on the `message` pointer. Callers query
//! [`dash_sdk_error_consensus_error_count`] and
//! [`dash_sdk_error_consensus_error_at`] *before* freeing the error with
//! [`dash_sdk_error_free`]; freeing also releases the sidecar entry.

use dash_sdk::dpp::consensus::{codes::ErrorWithCode, ConsensusError};
use dash_sdk::dpp::ProtocolError;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::{CString, NulError};
use std::os::raw::c_char;
use std::sync::Mutex;
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

/// Error structure returned by FFI functions.
///
/// # ABI
///
/// This struct is frozen for backwards compatibility — do not add or reorder
/// fields. To inspect structured protocol consensus errors associated with this
/// error, use [`dash_sdk_error_consensus_error_count`] and
/// [`dash_sdk_error_consensus_error_at`] before calling
/// [`dash_sdk_error_free`].
#[repr(C)]
pub struct DashSDKError {
    /// Error code
    pub code: DashSDKErrorCode,
    /// Human-readable error message (null-terminated C string)
    /// Caller must free this with dash_sdk_error_free
    pub message: *mut c_char,
}

/// Structured detail for a single protocol consensus error.
///
/// Returned by [`dash_sdk_error_consensus_error_at`]. Free each instance with
/// [`dash_sdk_consensus_error_free`].
#[repr(C)]
pub struct DashSDKConsensusError {
    /// Numeric consensus error code from DPP's `ErrorWithCode`.
    pub code: u32,
    /// High-level kind, e.g. `BasicError`, `StateError` (owned C string).
    pub kind: *mut c_char,
    /// Specific error name (currently mirrors `kind`, owned C string).
    pub name: *mut c_char,
    /// Human-readable message (owned C string).
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

#[derive(Debug, Clone)]
struct ConsensusErrorEntry {
    code: u32,
    kind: String,
    name: String,
    message: String,
}

/// Sidecar map from the `DashSDKError.message` raw pointer (as `usize`) to the
/// structured consensus error details. Populated when a `ProtocolError`
/// containing one or more `ConsensusError`s is converted into a
/// `DashSDKError`; freed by `dash_sdk_error_free`.
static CONSENSUS_ERROR_SIDECAR: Lazy<Mutex<HashMap<usize, Vec<ConsensusErrorEntry>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn register_consensus_errors(message_ptr: *mut c_char, errors: Vec<ConsensusErrorEntry>) {
    if message_ptr.is_null() || errors.is_empty() {
        return;
    }
    if let Ok(mut map) = CONSENSUS_ERROR_SIDECAR.lock() {
        map.insert(message_ptr as usize, errors);
    }
}

fn take_consensus_errors(message_ptr: *mut c_char) {
    if message_ptr.is_null() {
        return;
    }
    if let Ok(mut map) = CONSENSUS_ERROR_SIDECAR.lock() {
        map.remove(&(message_ptr as usize));
    }
}

fn with_consensus_errors<R>(
    message_ptr: *const c_char,
    f: impl FnOnce(&[ConsensusErrorEntry]) -> R,
) -> Option<R> {
    if message_ptr.is_null() {
        return None;
    }
    let guard = CONSENSUS_ERROR_SIDECAR.lock().ok()?;
    guard.get(&(message_ptr as usize)).map(|v| f(v.as_slice()))
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
                if let dash_sdk::Error::Protocol(protocol_error) = sdk_err {
                    if let Some((message, entries)) =
                        format_protocol_consensus_error(protocol_error)
                    {
                        let error = DashSDKError::new(DashSDKErrorCode::ProtocolError, message);
                        register_consensus_errors(error.message, entries);
                        return error;
                    }
                }

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

fn consensus_error_kind_name(error: &ConsensusError) -> &'static str {
    match error {
        ConsensusError::DefaultError => "DefaultError",
        ConsensusError::BasicError(_) => "BasicError",
        ConsensusError::StateError(_) => "StateError",
        ConsensusError::SignatureError(_) => "SignatureError",
        ConsensusError::FeeError(_) => "FeeError",
    }
}

fn consensus_error_entry(error: &ConsensusError) -> ConsensusErrorEntry {
    let name = consensus_error_kind_name(error).to_string();
    ConsensusErrorEntry {
        code: error.code(),
        kind: name.clone(),
        name,
        message: error.to_string(),
    }
}

fn format_protocol_consensus_error(
    error: &ProtocolError,
) -> Option<(String, Vec<ConsensusErrorEntry>)> {
    match error {
        ProtocolError::ConsensusError(consensus_error) => {
            let message = consensus_error.to_string();
            let entries = vec![consensus_error_entry(consensus_error)];
            Some((message, entries))
        }
        ProtocolError::ConsensusErrors(consensus_errors) => {
            let message = consensus_errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            let entries = consensus_errors.iter().map(consensus_error_entry).collect();
            Some((message, entries))
        }
        _ => None,
    }
}

/// Free an error message.
///
/// Also releases any structured consensus-error sidecar associated with the
/// error's message pointer, if one was attached.
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
        take_consensus_errors(error.message);
        let _ = CString::from_raw(error.message);
    }
}

/// Returns the number of structured protocol consensus errors associated with
/// `error`, or `0` if `error` is null, is not a `ProtocolError`, or carries no
/// structured details.
///
/// # Safety
/// - `error` must either be null or a pointer previously returned by this SDK
///   that has not yet been freed.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_error_consensus_error_count(error: *const DashSDKError) -> usize {
    if error.is_null() {
        return 0;
    }
    let error = &*error;
    if error.code != DashSDKErrorCode::ProtocolError {
        return 0;
    }
    with_consensus_errors(error.message, |entries| entries.len()).unwrap_or(0)
}

/// Returns a newly-allocated [`DashSDKConsensusError`] for the consensus error
/// at `index`, or null if `error` is null, is not a `ProtocolError`, has no
/// structured details, `index` is out of range, or memory allocation fails.
///
/// The returned pointer is owned by the caller and must be freed with
/// [`dash_sdk_consensus_error_free`].
///
/// # Safety
/// - `error` must either be null or a pointer previously returned by this SDK
///   that has not yet been freed.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_error_consensus_error_at(
    error: *const DashSDKError,
    index: usize,
) -> *mut DashSDKConsensusError {
    if error.is_null() {
        return std::ptr::null_mut();
    }
    let error = &*error;
    if error.code != DashSDKErrorCode::ProtocolError {
        return std::ptr::null_mut();
    }

    let entry =
        with_consensus_errors(error.message, |entries| entries.get(index).cloned()).flatten();
    let Some(entry) = entry else {
        return std::ptr::null_mut();
    };

    let kind = match CString::new(entry.kind) {
        Ok(s) => s.into_raw(),
        Err(_) => return std::ptr::null_mut(),
    };
    let name = match CString::new(entry.name) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            let _ = CString::from_raw(kind);
            return std::ptr::null_mut();
        }
    };
    let message = match CString::new(entry.message) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            let _ = CString::from_raw(kind);
            let _ = CString::from_raw(name);
            return std::ptr::null_mut();
        }
    };

    Box::into_raw(Box::new(DashSDKConsensusError {
        code: entry.code,
        kind,
        name,
        message,
    }))
}

/// Free a [`DashSDKConsensusError`] returned by
/// [`dash_sdk_error_consensus_error_at`].
///
/// # Safety
/// - `error` must be a pointer previously returned by
///   `dash_sdk_error_consensus_error_at`, or null (no-op).
/// - After this call, `error` becomes invalid and must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_consensus_error_free(error: *mut DashSDKConsensusError) {
    if error.is_null() {
        return;
    }
    let error = Box::from_raw(error);
    if !error.kind.is_null() {
        let _ = CString::from_raw(error.kind);
    }
    if !error.name.is_null() {
        let _ = CString::from_raw(error.name);
    }
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
    use dash_sdk::dpp::consensus::basic::document::NonceOutOfBoundsError;
    use dash_sdk::dpp::consensus::basic::token::InvalidTokenAmountError;
    use dash_sdk::dpp::consensus::{basic::BasicError, ConsensusError};
    use std::ffi::CStr;

    fn error_message(error: &DashSDKError) -> String {
        unsafe { CStr::from_ptr(error.message) }
            .to_str()
            .expect("ffi error message should be valid utf-8")
            .to_owned()
    }

    fn cstr(ptr: *mut c_char) -> String {
        unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .expect("c string should be valid utf-8")
            .to_owned()
    }

    /// Box and free via the public C ABI so the sidecar lifecycle exercised by
    /// real callers is exercised by the test.
    fn free_via_ffi(error: DashSDKError) {
        let raw = Box::into_raw(Box::new(error));
        unsafe { dash_sdk_error_free(raw) };
    }

    #[test]
    fn sdk_protocol_consensus_error_maps_to_protocol_error_code() {
        let consensus_error = ConsensusError::BasicError(BasicError::NonceOutOfBoundsError(
            NonceOutOfBoundsError::new(u64::MAX),
        ));
        let expected_code = consensus_error.code();
        let sdk_error =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus_error)));

        let ffi_error = DashSDKError::from(FFIError::SDKError(sdk_error));
        let message = error_message(&ffi_error);

        assert_eq!(ffi_error.code, DashSDKErrorCode::ProtocolError);
        assert!(message.contains("Nonce is out of bounds"));
        assert!(!message.contains("Failed to fetch balances"));

        // Structured sidecar exposes the singular consensus error.
        let count = unsafe { dash_sdk_error_consensus_error_count(&ffi_error) };
        assert_eq!(count, 1);

        let detail_ptr = unsafe { dash_sdk_error_consensus_error_at(&ffi_error, 0) };
        assert!(!detail_ptr.is_null());
        let detail = unsafe { &*detail_ptr };
        assert_eq!(detail.code, expected_code);
        assert_eq!(cstr(detail.kind), "BasicError");
        assert_eq!(cstr(detail.name), "BasicError");
        assert!(cstr(detail.message).contains("Nonce is out of bounds"));
        unsafe { dash_sdk_consensus_error_free(detail_ptr) };

        // Out-of-range index returns null.
        let oob = unsafe { dash_sdk_error_consensus_error_at(&ffi_error, 1) };
        assert!(oob.is_null());

        free_via_ffi(ffi_error);
    }

    #[test]
    fn sdk_protocol_consensus_errors_join_messages_readably() {
        let nonce_err = ConsensusError::BasicError(BasicError::NonceOutOfBoundsError(
            NonceOutOfBoundsError::new(u64::MAX),
        ));
        let token_err = ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
            InvalidTokenAmountError::new(100, 0),
        ));
        let expected_first_code = nonce_err.code();
        let expected_second_code = token_err.code();
        let sdk_error =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusErrors(vec![nonce_err, token_err]));

        let ffi_error = DashSDKError::from(FFIError::SDKError(sdk_error));
        let message = error_message(&ffi_error);

        assert_eq!(ffi_error.code, DashSDKErrorCode::ProtocolError);
        assert!(message.contains("Nonce is out of bounds"));
        assert!(message.contains("Invalid token amount 0"));
        assert!(message.contains("; "));
        assert!(!message.contains("Multiple consensus errors: ["));

        let count = unsafe { dash_sdk_error_consensus_error_count(&ffi_error) };
        assert_eq!(count, 2);

        let first_ptr = unsafe { dash_sdk_error_consensus_error_at(&ffi_error, 0) };
        let second_ptr = unsafe { dash_sdk_error_consensus_error_at(&ffi_error, 1) };
        assert!(!first_ptr.is_null() && !second_ptr.is_null());
        let first = unsafe { &*first_ptr };
        let second = unsafe { &*second_ptr };

        assert_eq!(cstr(first.kind), "BasicError");
        assert_eq!(cstr(first.name), "BasicError");
        assert!(cstr(first.message).contains("Nonce is out of bounds"));
        assert_eq!(first.code, expected_first_code);

        assert_eq!(cstr(second.kind), "BasicError");
        assert_eq!(cstr(second.name), "BasicError");
        assert!(cstr(second.message).contains("Invalid token amount 0"));
        assert_eq!(second.code, expected_second_code);

        unsafe { dash_sdk_consensus_error_free(first_ptr) };
        unsafe { dash_sdk_consensus_error_free(second_ptr) };

        free_via_ffi(ffi_error);
    }

    #[test]
    fn non_consensus_error_reports_zero_consensus_errors() {
        let ffi_error = DashSDKError::from(FFIError::NotFound("nope".to_string()));
        assert_eq!(ffi_error.code, DashSDKErrorCode::NotFound);

        let count = unsafe { dash_sdk_error_consensus_error_count(&ffi_error) };
        assert_eq!(count, 0);
        let null = unsafe { dash_sdk_error_consensus_error_at(&ffi_error, 0) };
        assert!(null.is_null());

        free_via_ffi(ffi_error);
    }

    #[test]
    fn null_error_is_safe() {
        let count = unsafe { dash_sdk_error_consensus_error_count(std::ptr::null()) };
        assert_eq!(count, 0);
        let null = unsafe { dash_sdk_error_consensus_error_at(std::ptr::null(), 0) };
        assert!(null.is_null());
    }
}
