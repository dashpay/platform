//! Error handling for FFI layer

use dash_sdk::dpp::consensus::codes::ErrorWithCode;
use dash_sdk::dpp::ProtocolError;
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

/// Which family a consensus rejection belongs to, paired with the numeric
/// `consensus_code` on [`DashSDKError`].
///
/// rs-dpp numbers its consensus errors by family
/// (`packages/rs-dpp/src/errors/consensus/codes.rs`): basic 1xxxx, signature
/// 2xxxx, fee 3xxxx, state 4xxxx. The values are those of
/// `PlatformWalletFFIConsensusErrorKind` in rs-platform-wallet-ffi, so a host
/// decodes the kind of either FFI's error the same way.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKConsensusErrorKind {
    /// Not a consensus rejection; pairs with `consensus_code == 0`
    ConsensusErrorKindNone = 0,
    /// Structure or version validation refused the transition
    ConsensusErrorKindBasic = 1,
    /// The transition's signature or signing key was refused
    ConsensusErrorKindSignature = 2,
    /// The fee could not be covered
    ConsensusErrorKindFee = 3,
    /// The transition was well formed but Platform state refused it
    ConsensusErrorKindState = 4,
}

impl DashSDKConsensusErrorKind {
    /// The family rs-dpp's numbering puts `code` in. `ConsensusErrorKindNone`
    /// for any other number: `ConsensusError::DefaultError` (1) names no
    /// rejection, and a wait-for-result failure that is not a consensus
    /// rejection carries a gRPC status code (1 to 16) instead.
    fn of_code(code: u32) -> Self {
        match code {
            10_000..=19_999 => Self::ConsensusErrorKindBasic,
            20_000..=29_999 => Self::ConsensusErrorKindSignature,
            30_000..=39_999 => Self::ConsensusErrorKindFee,
            40_000..=49_999 => Self::ConsensusErrorKindState,
            _ => Self::ConsensusErrorKindNone,
        }
    }
}

/// Error structure returned by FFI functions
///
/// `code` and `message` are the pair every host has always read.
/// `consensus_code` and `consensus_kind` name Platform's own rejection when
/// the failure was one, so a host can branch on the number instead of
/// matching the message. They are the same pair `PlatformWalletFFIResult`
/// carries. Rust allocates and frees every `DashSDKError` and hosts read it
/// through a pointer, so the two fields follow `message`.
#[repr(C)]
pub struct DashSDKError {
    /// Error code
    pub code: DashSDKErrorCode,
    /// Human-readable error message (null-terminated C string)
    /// Caller must free this with dash_sdk_error_free
    pub message: *mut c_char,
    /// The rs-dpp consensus error code Platform refused the operation with
    /// (10422 for a violated propertyConstraints rule, 41107 for a banned
    /// user), or 0 when the failure was not a consensus rejection. Real codes
    /// start at 10000, so 0 is unambiguous.
    pub consensus_code: u32,
    /// The family `consensus_code` belongs to, or `ConsensusErrorKindNone`
    /// when there is no code.
    pub consensus_kind: DashSDKConsensusErrorKind,
}

/// Internal error type for FFI operations
#[derive(Debug, Error)]
pub enum FFIError {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("SDK error: {0}")]
    SDKError(#[from] dash_sdk::Error),

    /// An SDK call failed. Renders and classifies as the
    /// `InternalError(format!("{context}: {source}"))` it replaces, and keeps
    /// `source` so the consensus code it carries reaches the host.
    #[error("Internal error: {context}: {source}")]
    SDKCallFailed {
        context: String,
        source: dash_sdk::Error,
    },

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
            consensus_code: 0,
            consensus_kind: DashSDKConsensusErrorKind::ConsensusErrorKindNone,
        }
    }

    /// Create a success result
    pub fn success() -> Self {
        DashSDKError {
            code: DashSDKErrorCode::Success,
            message: std::ptr::null_mut(),
            consensus_code: 0,
            consensus_kind: DashSDKConsensusErrorKind::ConsensusErrorKindNone,
        }
    }

    /// Stamp the consensus rejection `error` carries, if any, onto this error.
    ///
    /// Leaves `code` and `message` alone: hosts already branch on them, so the
    /// consensus code goes beside them rather than in place of them.
    pub fn with_consensus_error_of(mut self, error: &dash_sdk::Error) -> Self {
        if let Some(consensus_code) = consensus_code_of(error) {
            let kind = DashSDKConsensusErrorKind::of_code(consensus_code);
            if kind != DashSDKConsensusErrorKind::ConsensusErrorKindNone {
                self.consensus_code = consensus_code;
                self.consensus_kind = kind;
            }
        }
        self
    }
}

/// The consensus error code in the shapes a Platform refusal reaches the SDK
/// in: a CheckTx refusal decoded from the `dash-serialized-consensus-error-bin`
/// gRPC metadata (the same `Protocol(ConsensusError)` the SDK's own checks
/// return before broadcasting), a wait-for-result failure, and the retry
/// envelope around either. A wait-for-result failure keeps its `code` even
/// when its consensus error did not come with it.
fn consensus_code_of(error: &dash_sdk::Error) -> Option<u32> {
    match error {
        dash_sdk::Error::Protocol(ProtocolError::ConsensusError(consensus_error)) => {
            Some(consensus_error.code())
        }
        dash_sdk::Error::StateTransitionBroadcastError(broadcast_error) => Some(
            broadcast_error
                .cause
                .as_ref()
                .map_or(broadcast_error.code, |cause| cause.code()),
        ),
        dash_sdk::Error::NoAvailableAddressesToRetry(last_error) => consensus_code_of(last_error),
        _ => None,
    }
}

impl FFIError {
    /// `source` as an internal error with the message `"{context}: {source}"`,
    /// keeping the SDK error so its consensus code reaches the host.
    pub fn sdk_call_failed(context: impl Into<String>, source: dash_sdk::Error) -> Self {
        FFIError::SDKCallFailed {
            context: context.into(),
            source,
        }
    }
}

impl From<FFIError> for DashSDKError {
    fn from(err: FFIError) -> Self {
        let (code, message) = match &err {
            FFIError::InvalidParameter(_) => (DashSDKErrorCode::InvalidParameter, err.to_string()),
            FFIError::SDKCallFailed { .. } => (DashSDKErrorCode::InternalError, err.to_string()),
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

        let error = DashSDKError::new(code, message);
        match &err {
            FFIError::SDKError(source) | FFIError::SDKCallFailed { source, .. } => {
                error.with_consensus_error_of(source)
            }
            _ => error,
        }
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

    mod consensus_code {
        use super::*;
        use dash_sdk::dapi_client::transport::TransportError;
        use dash_sdk::dapi_client::DapiClientError;
        use dash_sdk::dapi_grpc::tonic::metadata::{MetadataMap, MetadataValue};
        use dash_sdk::dapi_grpc::tonic::{Code, Status};
        use dash_sdk::dpp::consensus::basic::document::{
            DocumentPropertyConstraintViolatedError, PropertyConstraintViolation,
        };
        use dash_sdk::dpp::consensus::state::contract_moderation::ContractUserBannedError;
        use dash_sdk::dpp::consensus::ConsensusError;
        use dash_sdk::dpp::platform_value::Identifier;
        use dash_sdk::dpp::serialization::PlatformSerializableWithPlatformVersion;
        use dash_sdk::dpp::version::PlatformVersion;
        use dash_sdk::error::StateTransitionBroadcastError;

        /// What a host reads from the error, with the message freed.
        struct Converted {
            code: DashSDKErrorCode,
            message: String,
            consensus_code: u32,
            consensus_kind: DashSDKConsensusErrorKind,
        }

        fn convert(err: FFIError) -> Converted {
            let error: DashSDKError = err.into();
            let message = unsafe {
                let message = std::ffi::CStr::from_ptr(error.message)
                    .to_string_lossy()
                    .into_owned();
                let _ = CString::from_raw(error.message);
                message
            };
            Converted {
                code: error.code,
                message,
                consensus_code: error.consensus_code,
                consensus_kind: error.consensus_kind,
            }
        }

        fn property_constraint_violated() -> ConsensusError {
            DocumentPropertyConstraintViolatedError::new(
                "post".to_string(),
                "rule 0".to_string(),
                PropertyConstraintViolation::NotMet,
            )
            .into()
        }

        fn contract_user_banned() -> ConsensusError {
            ContractUserBannedError::new(Identifier::new([1; 32]), Identifier::new([2; 32])).into()
        }

        /// What the SDK makes of DAPI refusing a transition at CheckTx: a gRPC
        /// status carrying the serialized consensus error in its metadata.
        fn refused_by_platform(consensus_error: &ConsensusError) -> dash_sdk::Error {
            let bytes = consensus_error
                .serialize_to_bytes_with_platform_version(PlatformVersion::latest())
                .expect("serialize consensus error");
            let mut metadata = MetadataMap::new();
            metadata.insert_bin(
                "dash-serialized-consensus-error-bin",
                MetadataValue::from_bytes(&bytes),
            );
            let status =
                Status::with_metadata(Code::InvalidArgument, consensus_error.to_string(), metadata);
            dash_sdk::Error::from(DapiClientError::Transport(TransportError::Grpc(status)))
        }

        /// What the SDK makes of a transition Platform refused at block
        /// execution: the wait-for-result error, with or without the decoded
        /// consensus error it was sent with.
        fn failed_in_block(code: u32, cause: Option<ConsensusError>) -> dash_sdk::Error {
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code,
                message: "refused".to_string(),
                cause,
            })
        }

        #[test]
        fn should_carry_the_code_of_a_consensus_error_platform_refused_the_transition_with() {
            let refused = refused_by_platform(&property_constraint_violated());
            let expected_message = refused.to_string();

            let error = convert(FFIError::SDKError(refused));

            assert_eq!(error.consensus_code, 10422);
            assert_eq!(
                error.consensus_kind,
                DashSDKConsensusErrorKind::ConsensusErrorKindBasic
            );
            // The code and message hosts already read are the ones they read before.
            assert_eq!(error.code, DashSDKErrorCode::ProtocolError);
            assert_eq!(error.message, expected_message);
        }

        #[test]
        fn should_carry_the_code_of_a_state_error_platform_refused_the_transition_with() {
            let error = convert(FFIError::SDKError(refused_by_platform(
                &contract_user_banned(),
            )));

            assert_eq!(error.consensus_code, 41107);
            assert_eq!(
                error.consensus_kind,
                DashSDKConsensusErrorKind::ConsensusErrorKindState
            );
        }

        #[test]
        fn should_carry_the_code_through_the_message_of_the_call_that_failed() {
            let context = "Failed to mint token and wait";
            let stringified = FFIError::InternalError(format!(
                "{}: {}",
                context,
                refused_by_platform(&contract_user_banned())
            ));
            let before = convert(stringified);

            let error = convert(FFIError::sdk_call_failed(
                context,
                refused_by_platform(&contract_user_banned()),
            ));

            assert_eq!(error.consensus_code, 41107);
            assert_eq!(
                error.consensus_kind,
                DashSDKConsensusErrorKind::ConsensusErrorKindState
            );
            // Same code and text as the stringified error it replaces, which
            // carried no consensus code.
            assert_eq!(error.code, before.code);
            assert_eq!(error.message, before.message);
            assert_eq!(before.consensus_code, 0);
        }

        #[test]
        fn should_carry_the_code_of_a_transition_platform_refused_in_a_block() {
            let with_cause = convert(FFIError::SDKError(failed_in_block(
                41107,
                Some(contract_user_banned()),
            )));
            let without_cause = convert(FFIError::SDKError(failed_in_block(40132, None)));

            assert_eq!(with_cause.consensus_code, 41107);
            assert_eq!(
                with_cause.consensus_kind,
                DashSDKConsensusErrorKind::ConsensusErrorKindState
            );
            assert_eq!(without_cause.consensus_code, 40132);
            assert_eq!(
                without_cause.consensus_kind,
                DashSDKConsensusErrorKind::ConsensusErrorKindState
            );
        }

        #[test]
        fn should_name_the_family_of_a_code_from_its_range() {
            use DashSDKConsensusErrorKind as Kind;

            let families = [
                (10_000, Kind::ConsensusErrorKindBasic),
                (10_422, Kind::ConsensusErrorKindBasic),
                (19_999, Kind::ConsensusErrorKindBasic),
                (20_000, Kind::ConsensusErrorKindSignature),
                (20_002, Kind::ConsensusErrorKindSignature),
                (29_999, Kind::ConsensusErrorKindSignature),
                (30_000, Kind::ConsensusErrorKindFee),
                (39_999, Kind::ConsensusErrorKindFee),
                (40_000, Kind::ConsensusErrorKindState),
                (41_107, Kind::ConsensusErrorKindState),
                (49_999, Kind::ConsensusErrorKindState),
            ];
            for (code, kind) in families {
                let error = convert(FFIError::SDKError(failed_in_block(code, None)));
                assert_eq!(error.consensus_code, code);
                assert_eq!(error.consensus_kind, kind, "code {code}");
            }

            // A number outside the four families is no consensus code at all.
            for code in [0, 1, 16, 9_999, 50_000] {
                let error = convert(FFIError::SDKError(failed_in_block(code, None)));
                assert_eq!(error.consensus_code, 0, "code {code}");
                assert_eq!(
                    error.consensus_kind,
                    Kind::ConsensusErrorKindNone,
                    "code {code}"
                );
            }
        }

        #[test]
        fn should_carry_the_code_of_the_last_refusal_once_retries_ran_out() {
            let exhausted = dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(
                refused_by_platform(&property_constraint_violated()),
            ));

            let error = convert(FFIError::SDKError(exhausted));

            assert_eq!(error.consensus_code, 10422);
            assert_eq!(
                error.consensus_kind,
                DashSDKConsensusErrorKind::ConsensusErrorKindBasic
            );
        }

        #[test]
        fn should_report_no_consensus_code_for_a_failure_that_is_not_a_consensus_refusal() {
            let not_refusals = [
                FFIError::SDKError(dash_sdk::Error::Generic("boom".to_string())),
                // A wait-for-result failure with a gRPC status code (13, Internal).
                FFIError::SDKError(failed_in_block(13, None)),
                // The placeholder consensus error names no rule.
                FFIError::SDKError(dash_sdk::Error::from(ConsensusError::DefaultError)),
                FFIError::InternalError("boom".to_string()),
                FFIError::NullPointer,
            ];

            for not_refusal in not_refusals {
                let error = convert(not_refusal);
                assert_eq!(error.consensus_code, 0, "{}", error.message);
                assert_eq!(
                    error.consensus_kind,
                    DashSDKConsensusErrorKind::ConsensusErrorKindNone,
                    "{}",
                    error.message
                );
            }
        }
    }
}
