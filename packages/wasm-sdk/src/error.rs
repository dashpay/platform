use dash_sdk::dpp::ProtocolError;
use dash_sdk::{error::StateTransitionBroadcastError, Error as SdkError};
use js_sys::Uint8Array;
use rs_dapi_client::CanRetry;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_dpp2::error::WasmDppError;

/// Structured error surfaced to JS consumers
#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WasmSdkErrorKind {
    // SDK error kinds
    Config,
    Drive,
    DriveProofError,
    Protocol,
    Proof,
    InvalidProvedResponse,
    DapiClientError,
    DapiMocksError,
    CoreError,
    MerkleBlockError,
    CoreClientError,
    MissingDependency,
    TotalCreditsNotFound,
    EpochNotFound,
    TimeoutReached,
    AlreadyExists,
    InvalidCreditTransfer,
    Generic,
    ContextProviderError,
    Cancelled,
    StaleNode,
    StateTransitionBroadcastError,
    WaitForStateTransitionResultFailedAfterBroadcast,
    NonceOverflow,
    IdentityNonceNotFound,
    DriveInternalError,

    // Local helper kinds
    InvalidArgument,
    SerializationError,
    NotFound,
    /// Surface-stable scaffolded API that hasn't been wired through
    /// the wasm-sdk layer yet. JS callers can branch on this kind
    /// (vs `Generic`) to detect "the API exists but execution waits
    /// on a follow-up" without parsing the message.
    NotImplemented,
}

/// Structured error surfaced to JS consumers
#[wasm_bindgen]
#[derive(thiserror::Error, Debug, Clone)]
#[error("{message}")]
pub struct WasmSdkError {
    kind: WasmSdkErrorKind,
    message: String,
    /// Optional numeric code for some errors (e.g., broadcast error code).
    code: i32,
    /// Indicates if the operation can be retried safely.
    is_retriable: bool,
    /// Raw transition hash for post-broadcast wait failures.
    transition_hash: Option<Vec<u8>>,
}

// wasm-bindgen getters defined below in the second impl block

impl WasmSdkError {
    fn new<M: Into<String>>(
        kind: WasmSdkErrorKind,
        message: M,
        code: Option<i32>,
        is_retriable: bool,
        transition_hash: Option<Vec<u8>>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            code: code.unwrap_or(-1),
            is_retriable,
            transition_hash,
        }
    }

    pub(crate) fn generic(message: impl Into<String>) -> Self {
        Self::new(WasmSdkErrorKind::Generic, message, None, false, None)
    }

    pub(crate) fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(
            WasmSdkErrorKind::InvalidArgument,
            message,
            None,
            false,
            None,
        )
    }

    pub(crate) fn serialization(message: impl Into<String>) -> Self {
        Self::new(
            WasmSdkErrorKind::SerializationError,
            message,
            None,
            false,
            None,
        )
    }

    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self::new(WasmSdkErrorKind::NotFound, message, None, false, None)
    }

    /// Construct a [`WasmSdkErrorKind::NotImplemented`] error for a
    /// scaffolded API. `api_name` is the JS-facing method name (e.g.
    /// `"getDocumentsAverage"`) — keep the message short so JS callers
    /// can branch on `kind` rather than message-match.
    ///
    /// `#[allow(dead_code)]` because all the previously-scaffolded
    /// SUM/AVG bindings now have real implementations; kept as a
    /// constructor for future scaffolded APIs so the
    /// `WasmSdkErrorKind::NotImplemented` variant (still serialized
    /// in [`WasmSdkErrorKind::Display`] at the bottom of this file)
    /// has a single canonical construction site.
    #[allow(dead_code)]
    pub(crate) fn not_implemented(api_name: impl Into<String>) -> Self {
        let api = api_name.into();
        Self::new(
            WasmSdkErrorKind::NotImplemented,
            format!(
                "{api}: scaffolded API not yet wired through the wasm-sdk \
                 layer. The rs-drive primitives are available; plumbing them \
                 up to the browser-facing API is the pending SDK fan-out \
                 follow-up."
            ),
            None,
            false,
            None,
        )
    }
}

impl From<SdkError> for WasmSdkError {
    fn from(err: SdkError) -> Self {
        use SdkError::*;
        let retriable = err.can_retry();
        match err {
            AlreadyExists(msg) => {
                Self::new(WasmSdkErrorKind::AlreadyExists, msg, None, retriable, None)
            }
            Config(msg) => Self::new(WasmSdkErrorKind::Config, msg, None, retriable, None),
            Drive(e) => Self::new(WasmSdkErrorKind::Drive, e.to_string(), None, retriable, None),
            DriveProofError(e, _proof, _block_info) => Self::new(
                WasmSdkErrorKind::DriveProofError,
                e.to_string(),
                None,
                retriable,
                None,
            ),
            Protocol(e) => {
                Self::new(WasmSdkErrorKind::Protocol, e.to_string(), None, retriable, None)
            }
            Proof(e) => Self::new(WasmSdkErrorKind::Proof, e.to_string(), None, retriable, None),
            InvalidProvedResponse(msg) => Self::new(
                WasmSdkErrorKind::InvalidProvedResponse,
                msg,
                None,
                retriable,
                None,
            ),
            DapiClientError(e) => Self::new(
                WasmSdkErrorKind::DapiClientError,
                e.to_string(),
                None,
                retriable,
                None,
            ),
            #[cfg(feature = "mocks")]
            DapiMocksError(e) => Self::new(
                WasmSdkErrorKind::DapiMocksError,
                e.to_string(),
                None,
                retriable,
                None,
            ),
            CoreError(e) => {
                Self::new(WasmSdkErrorKind::CoreError, e.to_string(), None, retriable, None)
            }
            MerkleBlockError(e) => Self::new(
                WasmSdkErrorKind::MerkleBlockError,
                e.to_string(),
                None,
                retriable,
                None,
            ),
            CoreClientError(e) => Self::new(
                WasmSdkErrorKind::CoreClientError,
                e.to_string(),
                None,
                retriable,
                None,
            ),
            MissingDependency(kind, id) => Self::new(
                WasmSdkErrorKind::MissingDependency,
                format!("Required {} not found: {}", kind, id),
                None,
                retriable,
                None,
            ),
            InvalidCreditTransfer(msg) => Self::new(
                WasmSdkErrorKind::InvalidCreditTransfer,
                msg,
                None,
                retriable,
                None,
            ),
            TotalCreditsNotFound => Self::new(
                WasmSdkErrorKind::TotalCreditsNotFound,
                "Total credits in Platform are not found; it should never happen".to_string(),
                None,
                retriable,
                None,
            ),
            EpochNotFound => Self::new(
                WasmSdkErrorKind::EpochNotFound,
                "No epoch found on Platform; it should never happen".to_string(),
                None,
                retriable,
                None,
            ),
            TimeoutReached(duration, msg) => Self::new(
                WasmSdkErrorKind::TimeoutReached,
                format!(
                    "SDK operation timeout {} secs reached: {}",
                    duration.as_secs(),
                    msg
                ),
                None,
                retriable,
                None,
            ),
            Generic(msg) => Self::new(WasmSdkErrorKind::Generic, msg, None, retriable, None),
            ContextProviderError(e) => Self::new(
                WasmSdkErrorKind::ContextProviderError,
                e.to_string(),
                None,
                retriable,
                None,
            ),
            Cancelled(msg) => {
                Self::new(WasmSdkErrorKind::Cancelled, msg, None, retriable, None)
            }
            StaleNode(e) => {
                Self::new(WasmSdkErrorKind::StaleNode, e.to_string(), None, retriable, None)
            }
            StateTransitionBroadcastError(e) => WasmSdkError::from(e),
            WaitForStateTransitionResultFailedAfterBroadcast {
                transition_hash,
                source,
            } => Self::new(
                WasmSdkErrorKind::WaitForStateTransitionResultFailedAfterBroadcast,
                format!(
                    "state transition broadcast succeeded for {} but waiting for the result failed: {}",
                    hex::encode(transition_hash),
                    source
                ),
                None,
                retriable,
                Some(transition_hash.to_vec()),
            ),
            NonceOverflow(nonce) => Self::new(
                WasmSdkErrorKind::NonceOverflow,
                format!(
                    "Identity nonce overflow: nonce has reached the maximum value ({})",
                    nonce
                ),
                None,
                false,
                None,
            ),
            IdentityNonceNotFound(msg) => {
                Self::new(
                    WasmSdkErrorKind::IdentityNonceNotFound,
                    msg,
                    None,
                    true,
                    None,
                )
            }
            DriveInternalError(msg) => Self::new(
                WasmSdkErrorKind::DriveInternalError,
                msg,
                None,
                retriable,
                None,
            ),
            NoAvailableAddressesToRetry(inner) => Self::new(
                WasmSdkErrorKind::DapiClientError,
                format!("no available addresses to retry, last error: {}", inner),
                None,
                retriable,
                None,
            ),
        }
    }
}
impl From<ProtocolError> for WasmSdkError {
    fn from(err: ProtocolError) -> Self {
        Self::new(
            WasmSdkErrorKind::Protocol,
            err.to_string(),
            None,
            false,
            None,
        )
    }
}

impl From<StateTransitionBroadcastError> for WasmSdkError {
    fn from(err: StateTransitionBroadcastError) -> Self {
        Self::new(
            WasmSdkErrorKind::StateTransitionBroadcastError,
            err.to_string(),
            Some(err.code as i32),
            false,
            None,
        )
    }
}

impl From<WasmDppError> for WasmSdkError {
    fn from(err: WasmDppError) -> Self {
        use wasm_dpp2::error::WasmDppErrorKind;
        // Map WasmDppError kind to appropriate WasmSdkError kind
        let kind = match err.kind() {
            WasmDppErrorKind::Protocol => WasmSdkErrorKind::Protocol,
            WasmDppErrorKind::InvalidArgument => WasmSdkErrorKind::InvalidArgument,
            WasmDppErrorKind::Serialization => WasmSdkErrorKind::SerializationError,
            WasmDppErrorKind::Conversion => WasmSdkErrorKind::SerializationError,
            WasmDppErrorKind::Generic => WasmSdkErrorKind::Generic,
        };
        Self::new(kind, err.to_string(), None, false, None)
    }
}

#[wasm_bindgen]
impl WasmSdkError {
    /// Error kind (enum)
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> WasmSdkErrorKind {
        self.kind
    }

    /// Backwards-compatible name string for the kind
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        use WasmSdkErrorKind as K;
        match self.kind {
            K::Config => "Config",
            K::Drive => "Drive",
            K::DriveProofError => "DriveProofError",
            K::Protocol => "Protocol",
            K::Proof => "Proof",
            K::InvalidProvedResponse => "InvalidProvedResponse",
            K::DapiClientError => "DapiClientError",
            K::DapiMocksError => "DapiMocksError",
            K::CoreError => "CoreError",
            K::MerkleBlockError => "MerkleBlockError",
            K::CoreClientError => "CoreClientError",
            K::MissingDependency => "MissingDependency",
            K::TotalCreditsNotFound => "TotalCreditsNotFound",
            K::EpochNotFound => "EpochNotFound",
            K::TimeoutReached => "TimeoutReached",
            K::AlreadyExists => "AlreadyExists",
            K::InvalidCreditTransfer => "InvalidCreditTransfer",
            K::Generic => "Generic",
            K::ContextProviderError => "ContextProviderError",
            K::Cancelled => "Cancelled",
            K::StaleNode => "StaleNode",
            K::StateTransitionBroadcastError => "StateTransitionBroadcastError",
            K::WaitForStateTransitionResultFailedAfterBroadcast => {
                "WaitForStateTransitionResultFailedAfterBroadcast"
            }
            K::NonceOverflow => "NonceOverflow",
            K::IdentityNonceNotFound => "IdentityNonceNotFound",
            K::DriveInternalError => "DriveInternalError",
            K::InvalidArgument => "InvalidArgument",
            K::SerializationError => "SerializationError",
            K::NotFound => "NotFound",
            K::NotImplemented => "NotImplemented",
        }
        .to_string()
    }

    /// Human-readable message
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// Optional numeric code. -1 means absent/not applicable
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> i32 {
        self.code
    }

    /// Whether the error is retryable
    #[wasm_bindgen(getter = "isRetriable")]
    pub fn is_retriable(&self) -> bool {
        self.is_retriable
    }

    /// Raw transition hash for post-broadcast wait failures.
    #[wasm_bindgen(getter, js_name = "transitionHash")]
    pub fn transition_hash(&self) -> Option<Uint8Array> {
        self.transition_hash
            .as_ref()
            .map(|bytes| Uint8Array::from(bytes.as_slice()))
    }
}

#[cfg(test)]
mod tests {
    use super::{WasmSdkError, WasmSdkErrorKind};
    use dash_sdk::Error as SdkError;
    use std::time::Duration;

    #[test]
    fn wait_error_conversion_preserves_transition_hash_bytes() {
        let transition_hash = [7_u8; 32];
        let sdk_error = SdkError::WaitForStateTransitionResultFailedAfterBroadcast {
            transition_hash,
            source: Box::new(SdkError::TimeoutReached(
                Duration::from_secs(1),
                "timed out".to_string(),
            )),
        };

        let wasm_error = WasmSdkError::from(sdk_error);

        assert_eq!(
            wasm_error.kind(),
            WasmSdkErrorKind::WaitForStateTransitionResultFailedAfterBroadcast
        );
        assert_eq!(wasm_error.transition_hash, Some(transition_hash.to_vec()));
        assert!(wasm_error.message().contains(&hex::encode(transition_hash)));
    }

    #[test]
    fn unrelated_errors_do_not_set_transition_hash() {
        let wasm_error = WasmSdkError::generic("boom");

        assert_eq!(wasm_error.transition_hash, None);
    }
}
