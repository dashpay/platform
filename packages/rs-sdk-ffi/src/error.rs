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
//! lookup keyed on the heap [`DashSDKError`] pointer returned to the FFI
//! caller. Callers query
//! [`dash_sdk_error_consensus_error_count`] and
//! [`dash_sdk_error_consensus_error_at`] *before* freeing the error with
//! [`dash_sdk_error_free`]; freeing also releases the sidecar entry. If an FFI
//! caller leaks a returned `DashSDKError` and never calls the matching free
//! function, the active sidecar entry is leaked for the same process lifetime
//! as the leaked error allocation. Long-running embedders must therefore treat
//! `dash_sdk_error_free` / `dash_sdk_result_free` as mandatory ownership
//! cleanup, not just message-string cleanup.
//!
//! # Sidecar contract (pointer-identity)
//!
//! The sidecar is keyed on the heap `*mut DashSDKError` pointer that the SDK
//! returns to the caller (the value of `DashSDKResult.error` or the raw error
//! pointer returned by an FFI call). Callers must observe the following rules:
//!
//! - Always pass the original `*mut DashSDKError` / `*const DashSDKError`
//!   pointer to [`dash_sdk_error_consensus_error_count`] /
//!   [`dash_sdk_error_consensus_error_at`]. Querying through a copy of the
//!   `DashSDKError` value (a separate stack/heap allocation that happens to
//!   share the same `message` pointer) returns no structured details.
//! - Structured consensus details must be queried synchronously before the
//!   error is freed with [`dash_sdk_error_free`]. Once freed, the sidecar
//!   entry is gone and the pointer value may be reused by future allocations.
//! - During construction (between `DashSDKError::from(FFIError::SDKError(..))`
//!   and the final `Box::into_raw`), pending sidecar entries are temporarily
//!   indexed by the message pointer; this is an implementation detail and is
//!   not exposed to FFI callers. Sidecar-capable errors — in particular those
//!   produced by the `From<FFIError>` impl from
//!   `FFIError::SDKError(dash_sdk::Error::Protocol(_))` — must be returned via
//!   [`box_dashsdk_error`] (directly or via [`DashSDKResult::error`] /
//!   [`ffi_result!`]) so the pending entry is promoted to a stable pointer
//!   key. Hand-crafted [`DashSDKError::new`] values with no pending sidecar
//!   entry (e.g. local validation errors built without a `From<FFIError>`
//!   conversion) are outside this contract and may be boxed directly; they
//!   simply have no structured details to expose.
//!
//! # Compatibility notes
//!
//! [`DashSDKError::message`] is always owned by the `DashSDKError` allocation
//! itself. Ownership is released only when [`dash_sdk_error_free`] reclaims the
//! outer error (or when an unboxed value is dropped in Rust). External callers
//! and in-crate tests must treat `message` as borrowed memory and must not
//! reclaim it manually with `CString::from_raw`.
//!
//! For in-crate construction, any `DashSDKError` produced by
//! `From<FFIError>` while a consensus sidecar is still pending must not be
//! `mem::forget`-ed or otherwise have `Drop` bypassed before boxing, because
//! doing so would leak both the owned message and the pending sidecar entry.
//! All sidecar-capable return paths should route through [`box_dashsdk_error`]
//! (directly or via [`DashSDKResult::error`] / [`ffi_result!`]).

use dash_sdk::dapi_client::DapiClientError;
use dash_sdk::dpp::consensus::{codes::ErrorWithCode, ConsensusError};
use dash_sdk::dpp::ProtocolError;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::{CString, NulError};
use std::os::raw::c_char;
use std::sync::{Mutex, MutexGuard};
use thiserror::Error;

/// Lock a sidecar mutex tolerating poisoning. A panic on another thread that
/// poisoned the mutex must not permanently disable sidecar lookup or cleanup
/// — silently dropping details for every subsequent error would be a worse
/// failure mode than continuing with a recovered guard.
fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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

/// Error structure returned by FFI functions.
///
/// # ABI
///
/// This struct is frozen for backwards compatibility — do not add or reorder
/// fields. To inspect structured protocol consensus errors associated with this
/// error, use [`dash_sdk_error_consensus_error_count`] and
/// [`dash_sdk_error_consensus_error_at`] before calling
/// [`dash_sdk_error_free`].
///
/// # Compatibility notes
///
/// `message` is Drop-owned / [`dash_sdk_error_free`]-owned memory. Consumers,
/// including in-crate tests, may read it through [`std::ffi::CStr`] while the
/// error is live, but must not take ownership with `CString::from_raw`.
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
    /// Specific consensus error variant name (owned C string).
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

/// Pending sidecar map keyed by the `DashSDKError.message` raw pointer
/// (as `usize`). Populated transiently during `From<FFIError>` conversion
/// while the resulting value-type `DashSDKError` is still being constructed;
/// drained by [`box_dashsdk_error`] when the error is boxed for FFI return,
/// or by [`DashSDKError::drop`] if the value is dropped before being boxed.
static PENDING_CONSENSUS_ERRORS: Lazy<Mutex<HashMap<usize, Vec<ConsensusErrorEntry>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Active sidecar entry keyed in [`ACTIVE_CONSENSUS_ERRORS`]. The
/// `message_ptr` is captured at boxing time and re-checked on lookup against
/// the current `DashSDKError.message`. This catches the common stale-pointer
/// reuse case where the heap allocation of a freed error is recycled for a
/// brand-new `DashSDKError`: the new value's `message` pointer will not match
/// the captured one, so we return no details rather than the previous error's
/// stale entries.
///
/// Limit: a true post-free read of a *dangling* pointer (where the underlying
/// memory has not yet been recycled and still happens to contain the original
/// `message` field) is undefined behavior at the FFI layer and is
/// indistinguishable from a valid live pointer at this layer; the
/// move-only sidecar contract documented at the module level forbids this
/// usage pattern but cannot be enforced from inside the SDK.
#[derive(Debug, Clone)]
struct ActiveSidecarEntry {
    /// `DashSDKError.message` value at the time of boxing.
    message_ptr: usize,
    entries: Vec<ConsensusErrorEntry>,
}

/// Active sidecar map keyed by the heap `*mut DashSDKError` pointer that is
/// handed back across the FFI boundary. Populated by [`box_dashsdk_error`];
/// freed by [`dash_sdk_error_free`]. Keying by the boxed `DashSDKError`
/// pointer means a copied-by-value `DashSDKError` (which has a different
/// pointer identity) cannot accidentally resolve another error's sidecar
/// entry — even if its `message` raw pointer happens to coincide due to
/// allocator reuse. Each entry also carries the original `message` pointer
/// so post-free pointer reuse for a different error is detected on lookup.
static ACTIVE_CONSENSUS_ERRORS: Lazy<Mutex<HashMap<usize, ActiveSidecarEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn register_pending_consensus_errors(message_ptr: *mut c_char, errors: Vec<ConsensusErrorEntry>) {
    if message_ptr.is_null() || errors.is_empty() {
        return;
    }
    let mut map = lock_recover(&PENDING_CONSENSUS_ERRORS);
    map.insert(message_ptr as usize, errors);
}

fn take_pending_consensus_errors(message_ptr: *mut c_char) -> Option<Vec<ConsensusErrorEntry>> {
    if message_ptr.is_null() {
        return None;
    }
    let mut map = lock_recover(&PENDING_CONSENSUS_ERRORS);
    map.remove(&(message_ptr as usize))
}

fn take_active_consensus_errors(error_ptr: *mut DashSDKError) {
    if error_ptr.is_null() {
        return;
    }
    let mut map = lock_recover(&ACTIVE_CONSENSUS_ERRORS);
    map.remove(&(error_ptr as usize));
}

fn with_active_consensus_errors<R>(
    error_ptr: *const DashSDKError,
    f: impl FnOnce(&[ConsensusErrorEntry]) -> R,
) -> Option<R> {
    if error_ptr.is_null() {
        return None;
    }
    let guard = lock_recover(&ACTIVE_CONSENSUS_ERRORS);
    let entry = guard.get(&(error_ptr as usize))?;
    // Only dereference caller memory after confirming the boxed error pointer
    // is still active in the sidecar map. This avoids touching arbitrary
    // non-null pointers that are not one of our live boxed errors, while still
    // checking that the current `message` field matches the value captured at
    // boxing time.
    let current_message = unsafe { (*error_ptr).message } as usize;
    if entry.message_ptr != current_message {
        // Pointer key matched but the message field doesn't match the value
        // we recorded at boxing time — almost certainly a recycled heap
        // allocation now occupied by a different error. Treat as no sidecar.
        return None;
    }
    Some(f(entry.entries.as_slice()))
}

/// Box a [`DashSDKError`] for return across the FFI boundary, promoting any
/// pending consensus-error sidecar entries (keyed by the error's `message`
/// pointer) to the active sidecar (keyed by the heap error pointer).
///
/// Sidecar-capable errors — those built via the `From<FFIError>` impl from
/// `FFIError::SDKError(dash_sdk::Error::Protocol(_))` — MUST go through this
/// helper (directly or via [`DashSDKResult::error`] / the [`ffi_result!`]
/// macro) so the pending sidecar is reachable through the pointer the caller
/// actually receives. Hand-crafted [`DashSDKError::new`] errors that have no
/// pending sidecar entry are outside this contract; boxing them with bare
/// `Box::into_raw` is sound (it just produces an error with no structured
/// details), though routing them through this helper is also fine and is the
/// recommended default to keep return paths uniform.
pub fn box_dashsdk_error(error: DashSDKError) -> *mut DashSDKError {
    let message_ptr = error.message;
    let raw = Box::into_raw(Box::new(error));
    if let Some(entries) = take_pending_consensus_errors(message_ptr) {
        let mut map = lock_recover(&ACTIVE_CONSENSUS_ERRORS);
        map.insert(
            raw as usize,
            ActiveSidecarEntry {
                message_ptr: message_ptr as usize,
                entries,
            },
        );
    }
    raw
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

/// Reclaim the owned `message` `CString` and drop any pending sidecar entry
/// keyed on the message pointer. This makes it safe to `drop` a
/// `DashSDKError` value built via `From<FFIError>` without ever boxing it
/// (e.g. test helpers, error-conversion paths that fail before reaching
/// [`box_dashsdk_error`]) — both the message allocation and the pending
/// sidecar entry are reclaimed instead of leaking.
///
/// `box_dashsdk_error` moves the error into a `Box` and uses `Box::into_raw`,
/// which suppresses Drop until [`dash_sdk_error_free`] runs `Box::from_raw`,
/// so successful boxing → free paths still drop exactly once. The active
/// sidecar (keyed on the heap pointer) is removed by `dash_sdk_error_free`
/// before the Drop runs.
///
/// Compatibility note: `Drop` owns `message`. External callers and tests must
/// not reclaim `message` separately with `CString::from_raw(error.message)`; free
/// the outer error through [`dash_sdk_error_free`] instead.
impl Drop for DashSDKError {
    fn drop(&mut self) {
        if !self.message.is_null() {
            // Drain any still-pending sidecar entry keyed on this message
            // pointer. After a successful `box_dashsdk_error` promotion this
            // is a no-op (the entry has already been moved to the active
            // map). When the value is dropped without boxing, this prevents
            // a leak that would later mis-attribute details to a recycled
            // message allocation.
            let _ = take_pending_consensus_errors(self.message);
            // SAFETY: `message` was allocated via `CString::into_raw` in
            // `DashSDKError::new`; reclaim the allocation exactly once.
            unsafe {
                let _ = CString::from_raw(self.message);
            }
            self.message = std::ptr::null_mut();
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
                        register_pending_consensus_errors(error.message, entries);
                        return error;
                    }
                }

                classify_sdk_error(sdk_err)
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

/// Map a non-`Protocol` `dash_sdk::Error` to an FFI `(code, message)` pair by
/// matching on the variant rather than scanning the formatted message.
///
/// Protocol consensus errors are handled separately by the caller (they carry
/// a structured sidecar). Variants we cannot meaningfully classify fall back
/// to `InternalError` with a neutral `"SDK error: ..."` prefix so we never
/// misattribute them to a specific operation.
fn classify_sdk_error(sdk_err: &dash_sdk::Error) -> (DashSDKErrorCode, String) {
    match sdk_err {
        // Non-consensus protocol errors still surface as ProtocolError; their
        // consensus sibling is handled by the caller before this point.
        dash_sdk::Error::Protocol(_) => (DashSDKErrorCode::ProtocolError, sdk_err.to_string()),
        dash_sdk::Error::TimeoutReached(_, _) => (DashSDKErrorCode::Timeout, sdk_err.to_string()),
        dash_sdk::Error::Cancelled(message) => (
            DashSDKErrorCode::Timeout,
            format!("Operation cancelled: {message}"),
        ),
        dash_sdk::Error::StaleNode(_) => (
            DashSDKErrorCode::NetworkError,
            format!("Stale node response: {sdk_err}. Retry the operation or try another server."),
        ),
        // No-address / exhausted-addresses paths get the explicit operator
        // hint message.
        dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses)
        | dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddressesToRetry(_))
        | dash_sdk::Error::NoAvailableAddressesToRetry(_) => (
            DashSDKErrorCode::NetworkError,
            "Cannot connect to network: No DAPI addresses configured. The SDK needs masternode quorum information to connect to the network.".to_string(),
        ),
        dash_sdk::Error::DapiClientError(_) => (
            DashSDKErrorCode::NetworkError,
            format!("DAPI error: {}", sdk_err),
        ),
        dash_sdk::Error::DriveInternalError(inner) => {
            (DashSDKErrorCode::DriveInternalError, inner.clone())
        }
        dash_sdk::Error::ContextProviderError(_) => (
            DashSDKErrorCode::NetworkError,
            format!("Context provider error: {}", sdk_err),
        ),
        dash_sdk::Error::CoreClientError(_) => (
            DashSDKErrorCode::NetworkError,
            format!("Core client error: {}", sdk_err),
        ),
        dash_sdk::Error::MissingDependency(_, _)
        | dash_sdk::Error::TotalCreditsNotFound
        | dash_sdk::Error::EpochNotFound
        | dash_sdk::Error::IdentityNonceNotFound(_) => {
            (DashSDKErrorCode::NotFound, sdk_err.to_string())
        }
        dash_sdk::Error::Config(_)
        | dash_sdk::Error::Drive(_)
        | dash_sdk::Error::DriveProofError(_, _, _)
        | dash_sdk::Error::Proof(_)
        | dash_sdk::Error::InvalidProvedResponse(_)
        | dash_sdk::Error::CoreError(_)
        | dash_sdk::Error::MerkleBlockError(_)
        | dash_sdk::Error::AlreadyExists(_)
        | dash_sdk::Error::InvalidCreditTransfer(_)
        | dash_sdk::Error::NonceOverflow(_)
        | dash_sdk::Error::Generic(_)
        | dash_sdk::Error::StateTransitionBroadcastError(_)
        | dash_sdk::Error::DapiMocksError(_) => (
            DashSDKErrorCode::InternalError,
            format!("SDK error: {}", sdk_err),
        ),
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

/// Resolve the specific variant identifier of a `ConsensusError`.
///
/// The inner consensus enums (`BasicError`, `StateError`, `SignatureError`,
/// `FeeError`) derive `strum::IntoStaticStr`, which generates a compile-time
/// `impl From<&Enum> for &'static str` from the enum's structure. Adding a
/// future variant to one of those enums therefore extends this mapping
/// automatically with the correct variant identifier; there is no
/// `Debug`-format parsing or `_` wildcard that could silently drift if a
/// variant is added or renamed.
fn consensus_error_variant_name(error: &ConsensusError) -> &'static str {
    match error {
        ConsensusError::DefaultError => "DefaultError",
        ConsensusError::BasicError(inner) => inner.into(),
        ConsensusError::StateError(inner) => inner.into(),
        ConsensusError::SignatureError(inner) => inner.into(),
        ConsensusError::FeeError(inner) => inner.into(),
    }
}

fn consensus_error_entry(error: &ConsensusError) -> ConsensusErrorEntry {
    ConsensusErrorEntry {
        code: error.code(),
        kind: consensus_error_kind_name(error).to_string(),
        name: consensus_error_variant_name(error).to_string(),
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
/// - Per the move-only sidecar contract documented at the module level, no
///   alias of `error` (including any copy of its `message` pointer) may be
///   used to query consensus details after this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_error_free(error: *mut DashSDKError) {
    if error.is_null() {
        return;
    }

    // Drop any active sidecar entry keyed on the heap pointer the caller
    // received. This must happen *before* `Box::from_raw` so the lookup uses
    // the same `*mut DashSDKError` value the caller saw.
    take_active_consensus_errors(error);

    // Reclaiming the box runs `DashSDKError::drop`, which frees the message
    // `CString` and clears any (no-op for boxed paths) pending sidecar entry.
    let _ = Box::from_raw(error);
}

/// Returns the number of structured protocol consensus errors associated with
/// `error`, or `0` if `error` is null, is not a `ProtocolError`, or carries no
/// structured details.
///
/// # Safety
/// - `error` must either be null or a pointer previously returned by this SDK
///   that has not yet been freed.
/// - Must be called synchronously, before [`dash_sdk_error_free`], on the
///   same `DashSDKError` value the SDK returned (not a copy/alias). See the
///   module-level move-only sidecar contract.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_error_consensus_error_count(error: *const DashSDKError) -> usize {
    // Do not read fields from `error` before sidecar lookup: callers may pass
    // a stale pointer, and the sidecar miss path must avoid dereferencing it.
    // Active sidecar entries are only registered for ProtocolError consensus
    // details, so the previous code check is redundant once lookup succeeds.
    with_active_consensus_errors(error, |entries| entries.len()).unwrap_or(0)
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
/// - Must be called synchronously, before [`dash_sdk_error_free`], on the
///   same `DashSDKError` value the SDK returned (not a copy/alias). See the
///   module-level move-only sidecar contract.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_error_consensus_error_at(
    error: *const DashSDKError,
    index: usize,
) -> *mut DashSDKConsensusError {
    // Sidecar lookup comes before any field read from `error`; a miss returns
    // null without dereferencing the caller-provided pointer. Active entries
    // are only registered for ProtocolError consensus details.
    let entry =
        with_active_consensus_errors(error, |entries| entries.get(index).cloned()).flatten();
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
                return $crate::box_dashsdk_error(error);
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::consensus::basic::document::NonceOutOfBoundsError;
    use dash_sdk::dpp::consensus::basic::token::InvalidTokenAmountError;
    use dash_sdk::dpp::consensus::fee::balance_is_not_enough_error::BalanceIsNotEnoughError;
    use dash_sdk::dpp::consensus::fee::fee_error::FeeError;
    use dash_sdk::dpp::consensus::signature::{
        IdentityNotFoundError, SignatureError as DppSignatureError,
    };
    use dash_sdk::dpp::consensus::state::identity::IdentityAlreadyExistsError;
    use dash_sdk::dpp::consensus::state::state_error::StateError;
    use dash_sdk::dpp::consensus::{basic::BasicError, ConsensusError};
    use std::ffi::CStr;

    fn error_message_ptr(error: *const DashSDKError) -> String {
        unsafe { CStr::from_ptr((*error).message) }
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

    /// Box the error via the same helper used by real FFI return paths so the
    /// pending sidecar entries are promoted to the active map keyed by the
    /// heap `*mut DashSDKError` pointer.
    fn boxed(error: DashSDKError) -> *mut DashSDKError {
        box_dashsdk_error(error)
    }

    fn classify(err: dash_sdk::Error) -> DashSDKErrorCode {
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(err)));
        let code = unsafe { (*ffi_error).code };
        unsafe { dash_sdk_error_free(ffi_error) };
        code
    }

    #[test]
    fn sdk_protocol_consensus_error_maps_to_protocol_error_code() {
        let consensus_error = ConsensusError::BasicError(BasicError::NonceOutOfBoundsError(
            NonceOutOfBoundsError::new(u64::MAX),
        ));
        let expected_code = consensus_error.code();
        let sdk_error =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus_error)));

        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        let message = error_message_ptr(ffi_error);

        assert_eq!(
            unsafe { (*ffi_error).code },
            DashSDKErrorCode::ProtocolError
        );
        assert!(message.contains("Nonce is out of bounds"));
        assert!(!message.contains("Failed to fetch balances"));

        // Structured sidecar exposes the singular consensus error.
        let count = unsafe { dash_sdk_error_consensus_error_count(ffi_error) };
        assert_eq!(count, 1);

        let detail_ptr = unsafe { dash_sdk_error_consensus_error_at(ffi_error, 0) };
        assert!(!detail_ptr.is_null());
        let detail = unsafe { &*detail_ptr };
        assert_eq!(detail.code, expected_code);
        assert_eq!(cstr(detail.kind), "BasicError");
        assert_eq!(cstr(detail.name), "NonceOutOfBoundsError");
        assert!(cstr(detail.message).contains("Nonce is out of bounds"));
        unsafe { dash_sdk_consensus_error_free(detail_ptr) };

        // Out-of-range index returns null.
        let oob = unsafe { dash_sdk_error_consensus_error_at(ffi_error, 1) };
        assert!(oob.is_null());

        unsafe { dash_sdk_error_free(ffi_error) };
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

        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        let message = error_message_ptr(ffi_error);

        assert_eq!(
            unsafe { (*ffi_error).code },
            DashSDKErrorCode::ProtocolError
        );
        assert!(message.contains("Nonce is out of bounds"));
        assert!(message.contains("Invalid token amount 0"));
        assert!(message.contains("; "));
        assert!(!message.contains("Multiple consensus errors: ["));

        let count = unsafe { dash_sdk_error_consensus_error_count(ffi_error) };
        assert_eq!(count, 2);

        let first_ptr = unsafe { dash_sdk_error_consensus_error_at(ffi_error, 0) };
        let second_ptr = unsafe { dash_sdk_error_consensus_error_at(ffi_error, 1) };
        assert!(!first_ptr.is_null() && !second_ptr.is_null());
        let first = unsafe { &*first_ptr };
        let second = unsafe { &*second_ptr };

        assert_eq!(cstr(first.kind), "BasicError");
        assert_eq!(cstr(first.name), "NonceOutOfBoundsError");
        assert!(cstr(first.message).contains("Nonce is out of bounds"));
        assert_eq!(first.code, expected_first_code);

        assert_eq!(cstr(second.kind), "BasicError");
        assert_eq!(cstr(second.name), "InvalidTokenAmountError");
        assert!(cstr(second.message).contains("Invalid token amount 0"));
        assert_eq!(second.code, expected_second_code);

        unsafe { dash_sdk_consensus_error_free(first_ptr) };
        unsafe { dash_sdk_consensus_error_free(second_ptr) };

        unsafe { dash_sdk_error_free(ffi_error) };
    }

    /// Pointer-identity contract: a copied `DashSDKError` value that happens
    /// to share the same `message` pointer as a boxed error MUST NOT expose
    /// the boxed error's structured sidecar entries. Only the original boxed
    /// `*mut DashSDKError` resolves the active sidecar.
    #[test]
    fn copied_error_value_does_not_resolve_sidecar() {
        let consensus_error = ConsensusError::BasicError(BasicError::NonceOutOfBoundsError(
            NonceOutOfBoundsError::new(u64::MAX),
        ));
        let sdk_error =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus_error)));

        let original = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));

        // Original boxed pointer exposes the sidecar.
        assert_eq!(
            unsafe { dash_sdk_error_consensus_error_count(original) },
            1,
            "original boxed pointer must expose the sidecar"
        );

        // Construct a value-copy of the error that shares the same `message`
        // pointer. Querying through this copy must NOT resolve the sidecar
        // (different pointer identity).
        let copy = DashSDKError {
            code: unsafe { (*original).code },
            message: unsafe { (*original).message },
        };
        assert_eq!(
            unsafe { dash_sdk_error_consensus_error_count(&copy) },
            0,
            "copy must not resolve sidecar via shared message pointer"
        );
        let null = unsafe { dash_sdk_error_consensus_error_at(&copy, 0) };
        assert!(null.is_null(), "copy must not return any structured detail");
        // The copy aliases the original's owned `message` — `forget` it so
        // its `Drop` does not double-free; `dash_sdk_error_free(original)`
        // below releases the allocation.
        std::mem::forget(copy);

        unsafe { dash_sdk_error_free(original) };
    }

    /// Dropping a `DashSDKError` produced by `From<FFIError>` without ever
    /// boxing it must clear the pending sidecar entry keyed on its message
    /// pointer. Otherwise a later allocation that recycles the same address
    /// could pick up stale consensus-error details.
    #[test]
    fn dropping_unboxed_protocol_error_clears_pending_sidecar() {
        let consensus_error = ConsensusError::BasicError(BasicError::NonceOutOfBoundsError(
            NonceOutOfBoundsError::new(u64::MAX),
        ));
        let sdk_error =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus_error)));

        // Build the unboxed error and capture its message pointer key. While
        // the value is alive, the pending sidecar map should hold an entry
        // for this key.
        let error = DashSDKError::from(FFIError::SDKError(sdk_error));
        let message_key = error.message as usize;
        assert!(
            lock_recover(&PENDING_CONSENSUS_ERRORS).contains_key(&message_key),
            "From<FFIError> must register a pending sidecar entry"
        );

        // Dropping without boxing must reclaim the entry (and the CString),
        // not leak it.
        drop(error);
        assert!(
            !lock_recover(&PENDING_CONSENSUS_ERRORS).contains_key(&message_key),
            "Drop must remove pending sidecar entry"
        );

        // A subsequent boxed protocol error must show only its own details
        // even if the prior allocation is reused by the allocator.
        let next_consensus = ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
            InvalidTokenAmountError::new(7, 0),
        ));
        let next_sdk_error =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(next_consensus)));
        let next_ffi = boxed(DashSDKError::from(FFIError::SDKError(next_sdk_error)));
        let count = unsafe { dash_sdk_error_consensus_error_count(next_ffi) };
        assert_eq!(count, 1, "fresh error must report exactly its own details");
        let detail_ptr = unsafe { dash_sdk_error_consensus_error_at(next_ffi, 0) };
        assert!(!detail_ptr.is_null());
        let detail = unsafe { &*detail_ptr };
        assert_eq!(cstr(detail.name), "InvalidTokenAmountError");
        unsafe { dash_sdk_consensus_error_free(detail_ptr) };
        unsafe { dash_sdk_error_free(next_ffi) };
    }

    /// If a recycled `*mut DashSDKError` allocation is subsequently occupied
    /// by a different error, the active sidecar lookup must reject the stale
    /// entry rather than mis-attributing details. The mitigation re-checks
    /// the `message` pointer against the value captured at boxing time.
    #[test]
    fn active_sidecar_rejects_message_pointer_mismatch() {
        let consensus_error = ConsensusError::BasicError(BasicError::NonceOutOfBoundsError(
            NonceOutOfBoundsError::new(u64::MAX),
        ));
        let sdk_error =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus_error)));
        let original = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        let original_key = original as usize;

        // Simulate post-free pointer reuse: forge an active-sidecar entry
        // under `original`'s key whose recorded message pointer does NOT
        // match `original`'s current message pointer. Lookup must reject it.
        {
            let mut map = lock_recover(&ACTIVE_CONSENSUS_ERRORS);
            map.insert(
                original_key,
                ActiveSidecarEntry {
                    message_ptr: 0xdead_beef_usize,
                    entries: vec![ConsensusErrorEntry {
                        code: 9999,
                        kind: "BasicError".to_string(),
                        name: "BogusError".to_string(),
                        message: "stale entry from a freed predecessor".to_string(),
                    }],
                },
            );
        }

        // The forged entry has the wrong message pointer, so the count must
        // come back as 0 even though a key match exists.
        let count = unsafe { dash_sdk_error_consensus_error_count(original) };
        assert_eq!(
            count, 0,
            "stale entry with mismatched message pointer must be rejected"
        );
        let null = unsafe { dash_sdk_error_consensus_error_at(original, 0) };
        assert!(null.is_null(), "lookup must return no structured details");

        unsafe { dash_sdk_error_free(original) };
    }

    #[test]
    fn non_consensus_error_reports_zero_consensus_errors() {
        let ffi_error = boxed(DashSDKError::from(FFIError::NotFound("nope".to_string())));
        assert_eq!(unsafe { (*ffi_error).code }, DashSDKErrorCode::NotFound);

        let count = unsafe { dash_sdk_error_consensus_error_count(ffi_error) };
        assert_eq!(count, 0);
        let null = unsafe { dash_sdk_error_consensus_error_at(ffi_error, 0) };
        assert!(null.is_null());

        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn unclassified_sdk_error_uses_neutral_internal_fallback() {
        // `Generic` is a non-protocol SDK error variant whose Display string
        // does not match any of the timeout/network/DAPI/protocol/not-found
        // heuristics, so it exercises the neutral fallback branch.
        let sdk_error = dash_sdk::Error::Generic("widget exploded".to_string());
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        let message = error_message_ptr(ffi_error);

        assert_eq!(
            unsafe { (*ffi_error).code },
            DashSDKErrorCode::InternalError
        );
        assert!(
            !message.contains("Failed to fetch balances"),
            "neutral fallback must not reference fetch-balances; got: {}",
            message
        );
        assert!(
            message.starts_with("SDK error:"),
            "neutral fallback should be prefixed with 'SDK error:'; got: {}",
            message
        );
        assert!(message.contains("widget exploded"));

        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn timeout_reached_maps_to_timeout_code_structurally() {
        let sdk_error = dash_sdk::Error::TimeoutReached(
            std::time::Duration::from_secs(5),
            "fetching identity".to_string(),
        );
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        assert_eq!(unsafe { (*ffi_error).code }, DashSDKErrorCode::Timeout);
        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn cancelled_maps_to_timeout_with_clear_message() {
        let sdk_error = dash_sdk::Error::Cancelled("request aborted by caller".to_string());
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        assert_eq!(unsafe { (*ffi_error).code }, DashSDKErrorCode::Timeout);
        let message = error_message_ptr(ffi_error);
        assert!(
            message.contains("Operation cancelled"),
            "expected cancellation prefix, got: {message}"
        );
        assert!(
            message.contains("request aborted by caller"),
            "expected original cancellation reason, got: {message}"
        );
        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn stale_node_maps_to_network_with_retry_hint() {
        let sdk_error = dash_sdk::Error::StaleNode(dash_sdk::error::StaleNodeError::Height {
            expected_height: 100,
            received_height: 95,
            tolerance_blocks: 2,
        });
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        assert_eq!(unsafe { (*ffi_error).code }, DashSDKErrorCode::NetworkError);
        let message = error_message_ptr(ffi_error);
        assert!(
            message.contains("Stale node response"),
            "expected stale-node prefix, got: {message}"
        );
        assert!(
            message.contains("try another server") || message.contains("Retry the operation"),
            "expected retry guidance, got: {message}"
        );
        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn dapi_no_available_addresses_maps_to_network_with_hint() {
        let sdk_error = dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses);
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        assert_eq!(unsafe { (*ffi_error).code }, DashSDKErrorCode::NetworkError);
        let message = error_message_ptr(ffi_error);
        assert!(
            message.contains("No DAPI addresses configured"),
            "expected operator hint, got: {message}"
        );
        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn context_provider_error_maps_to_network_with_clear_prefix() {
        let sdk_error = dash_sdk::Error::ContextProviderError(
            dash_sdk::error::ContextProviderError::Generic("quorum lookup failed".to_string()),
        );
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        assert_eq!(unsafe { (*ffi_error).code }, DashSDKErrorCode::NetworkError);
        let message = error_message_ptr(ffi_error);
        assert!(message.starts_with("Context provider error:"));
        assert!(message.contains("quorum lookup failed"));
        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn drive_internal_error_with_not_found_substring_maps_to_drive_internal_error() {
        // Typed-variant matching must take precedence over message substring
        // heuristics such as "not found".
        let sdk_error =
            dash_sdk::Error::DriveInternalError("data contract not found 0x123".to_string());
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));

        assert_eq!(
            unsafe { (*ffi_error).code },
            DashSDKErrorCode::DriveInternalError
        );
        let message = error_message_ptr(ffi_error);
        assert_eq!(message, "data contract not found 0x123");

        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn drive_internal_error_plain_maps_to_drive_internal_error() {
        let sdk_error = dash_sdk::Error::DriveInternalError("storage layer constraint".to_string());
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));

        assert_eq!(
            unsafe { (*ffi_error).code },
            DashSDKErrorCode::DriveInternalError
        );
        let message = error_message_ptr(ffi_error);
        assert_eq!(message, "storage layer constraint");

        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn missing_dependency_maps_to_not_found_structurally() {
        let sdk_error =
            dash_sdk::Error::MissingDependency("data contract".to_string(), "abc123".to_string());
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        assert_eq!(unsafe { (*ffi_error).code }, DashSDKErrorCode::NotFound);
        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn epoch_not_found_maps_to_not_found_structurally() {
        let sdk_error = dash_sdk::Error::EpochNotFound;
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));
        assert_eq!(unsafe { (*ffi_error).code }, DashSDKErrorCode::NotFound);
        unsafe { dash_sdk_error_free(ffi_error) };
    }

    #[test]
    fn null_error_is_safe() {
        let count = unsafe { dash_sdk_error_consensus_error_count(std::ptr::null()) };
        assert_eq!(count, 0);
        let null = unsafe { dash_sdk_error_consensus_error_at(std::ptr::null(), 0) };
        assert!(null.is_null());
    }

    /// Representative variant-name extraction for `StateError`. Uses
    /// `IdentityAlreadyExistsError`, a constructible state-error variant.
    #[test]
    fn state_error_extracts_specific_variant_name() {
        let consensus_error = ConsensusError::StateError(StateError::IdentityAlreadyExistsError(
            IdentityAlreadyExistsError::new(Default::default()),
        ));
        let expected_code = consensus_error.code();
        let sdk_error =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus_error)));

        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));

        assert_eq!(
            unsafe { dash_sdk_error_consensus_error_count(ffi_error) },
            1
        );
        let detail_ptr = unsafe { dash_sdk_error_consensus_error_at(ffi_error, 0) };
        assert!(!detail_ptr.is_null());
        let detail = unsafe { &*detail_ptr };
        assert_eq!(detail.code, expected_code);
        assert_eq!(cstr(detail.kind), "StateError");
        assert_eq!(cstr(detail.name), "IdentityAlreadyExistsError");
        unsafe { dash_sdk_consensus_error_free(detail_ptr) };
        unsafe { dash_sdk_error_free(ffi_error) };
    }

    /// Representative variant-name extraction for `SignatureError`. Uses
    /// `IdentityNotFoundError`, a constructible signature-error variant.
    #[test]
    fn signature_error_extracts_specific_variant_name() {
        let consensus_error =
            ConsensusError::SignatureError(DppSignatureError::IdentityNotFoundError(
                IdentityNotFoundError::new(Default::default()),
            ));
        let expected_code = consensus_error.code();
        let sdk_error =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus_error)));

        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));

        assert_eq!(
            unsafe { dash_sdk_error_consensus_error_count(ffi_error) },
            1
        );
        let detail_ptr = unsafe { dash_sdk_error_consensus_error_at(ffi_error, 0) };
        assert!(!detail_ptr.is_null());
        let detail = unsafe { &*detail_ptr };
        assert_eq!(detail.code, expected_code);
        assert_eq!(cstr(detail.kind), "SignatureError");
        assert_eq!(cstr(detail.name), "IdentityNotFoundError");
        unsafe { dash_sdk_consensus_error_free(detail_ptr) };
        unsafe { dash_sdk_error_free(ffi_error) };
    }

    /// Representative variant-name extraction for `FeeError`. Uses
    /// `BalanceIsNotEnoughError`, a constructible fee-error variant.
    #[test]
    fn fee_error_extracts_specific_variant_name() {
        let consensus_error = ConsensusError::FeeError(FeeError::BalanceIsNotEnoughError(
            BalanceIsNotEnoughError::new(0, 1),
        ));
        let expected_code = consensus_error.code();
        let sdk_error =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus_error)));

        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(sdk_error)));

        assert_eq!(
            unsafe { dash_sdk_error_consensus_error_count(ffi_error) },
            1
        );
        let detail_ptr = unsafe { dash_sdk_error_consensus_error_at(ffi_error, 0) };
        assert!(!detail_ptr.is_null());
        let detail = unsafe { &*detail_ptr };
        assert_eq!(detail.code, expected_code);
        assert_eq!(cstr(detail.kind), "FeeError");
        assert_eq!(cstr(detail.name), "BalanceIsNotEnoughError");
        unsafe { dash_sdk_consensus_error_free(detail_ptr) };
        unsafe { dash_sdk_error_free(ffi_error) };
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
        let ffi_error = boxed(DashSDKError::from(FFIError::SDKError(err)));
        let rendered = error_message_ptr(ffi_error);

        assert_eq!(
            unsafe { (*ffi_error).code },
            DashSDKErrorCode::InternalError
        );
        assert!(rendered.starts_with("SDK error:"));
        assert!(rendered.contains("Proof verification error"));
        assert!(!rendered.contains("Failed to fetch balances"));
        unsafe { dash_sdk_error_free(ffi_error) };
    }
}
