use dash_sdk::dpp::consensus::codes::ErrorWithCode;
use dash_sdk::dpp::ProtocolError;
use dash_sdk::{error::StateTransitionBroadcastError, Error as SdkError};
use js_sys::{Array, Object, Reflect};
use rs_dapi_client::CanRetry;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
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
    ExecutionNotProved,
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
    /// Optional machine-readable details for JS consumers.
    details: JsValue,
}

// wasm-bindgen getters defined below in the second impl block

impl WasmSdkError {
    fn new<M: Into<String>>(
        kind: WasmSdkErrorKind,
        message: M,
        code: Option<i32>,
        is_retriable: bool,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            code: code.unwrap_or(-1),
            is_retriable,
            details: JsValue::UNDEFINED,
        }
    }

    pub(crate) fn generic(message: impl Into<String>) -> Self {
        Self::new(WasmSdkErrorKind::Generic, message, None, false)
    }

    pub(crate) fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(WasmSdkErrorKind::InvalidArgument, message, None, false)
    }

    pub(crate) fn serialization(message: impl Into<String>) -> Self {
        Self::new(WasmSdkErrorKind::SerializationError, message, None, false)
    }

    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self::new(WasmSdkErrorKind::NotFound, message, None, false)
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
        )
    }
}

impl From<dash_sdk::dash_platform_queries::Error> for WasmSdkError {
    fn from(err: dash_sdk::dash_platform_queries::Error) -> Self {
        // Route through the SDK's own conversion so the transport-free query
        // core's errors keep the exact mapping they had when they were
        // `SdkError` variants.
        SdkError::from(err).into()
    }
}

impl From<SdkError> for WasmSdkError {
    fn from(err: SdkError) -> Self {
        use SdkError::*;
        let retriable = err.can_retry();
        match err {
            AlreadyExists(msg) => Self::new(WasmSdkErrorKind::AlreadyExists, msg, None, retriable),
            Config(msg) => Self::new(WasmSdkErrorKind::Config, msg, None, retriable),
            Drive(e) => Self::new(WasmSdkErrorKind::Drive, e.to_string(), None, retriable),
            DriveProofError(e, _proof, _block_info) => Self::new(
                WasmSdkErrorKind::DriveProofError,
                e.to_string(),
                None,
                retriable,
            ),
            Protocol(e) => WasmSdkError::from(e),
            Proof(e) => Self::new(WasmSdkErrorKind::Proof, e.to_string(), None, retriable),
            // Deterministic for a given transition family: retrying another
            // node cannot upgrade a snapshot into execution evidence.
            ExecutionNotProved(msg) => {
                Self::new(WasmSdkErrorKind::ExecutionNotProved, msg, None, false)
            }
            InvalidProvedResponse(msg) => Self::new(
                WasmSdkErrorKind::InvalidProvedResponse,
                msg,
                None,
                retriable,
            ),
            DapiClientError(e) => Self::new(
                WasmSdkErrorKind::DapiClientError,
                e.to_string(),
                None,
                retriable,
            ),
            #[cfg(feature = "mocks")]
            DapiMocksError(e) => Self::new(
                WasmSdkErrorKind::DapiMocksError,
                e.to_string(),
                None,
                retriable,
            ),
            CoreError(e) => Self::new(WasmSdkErrorKind::CoreError, e.to_string(), None, retriable),
            MerkleBlockError(e) => Self::new(
                WasmSdkErrorKind::MerkleBlockError,
                e.to_string(),
                None,
                retriable,
            ),
            CoreClientError(e) => Self::new(
                WasmSdkErrorKind::CoreClientError,
                e.to_string(),
                None,
                retriable,
            ),
            MissingDependency(kind, id) => Self::new(
                WasmSdkErrorKind::MissingDependency,
                format!("Required {} not found: {}", kind, id),
                None,
                retriable,
            ),
            InvalidCreditTransfer(msg) => Self::new(
                WasmSdkErrorKind::InvalidCreditTransfer,
                msg,
                None,
                retriable,
            ),
            TotalCreditsNotFound => Self::new(
                WasmSdkErrorKind::TotalCreditsNotFound,
                "Total credits in Platform are not found; it should never happen".to_string(),
                None,
                retriable,
            ),
            EpochNotFound => Self::new(
                WasmSdkErrorKind::EpochNotFound,
                "No epoch found on Platform; it should never happen".to_string(),
                None,
                retriable,
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
            ),
            Generic(msg) => Self::new(WasmSdkErrorKind::Generic, msg, None, retriable),
            ContextProviderError(e) => Self::new(
                WasmSdkErrorKind::ContextProviderError,
                e.to_string(),
                None,
                retriable,
            ),
            Cancelled(msg) => Self::new(WasmSdkErrorKind::Cancelled, msg, None, retriable),
            StaleNode(e) => Self::new(WasmSdkErrorKind::StaleNode, e.to_string(), None, retriable),
            StateTransitionBroadcastError(e) => WasmSdkError::from(e),
            NonceOverflow(nonce) => Self::new(
                WasmSdkErrorKind::NonceOverflow,
                format!(
                    "Identity nonce overflow: nonce has reached the maximum value ({})",
                    nonce
                ),
                None,
                false,
            ),
            IdentityNonceNotFound(msg) => {
                Self::new(WasmSdkErrorKind::IdentityNonceNotFound, msg, None, true)
            }
            DriveInternalError(msg) => {
                Self::new(WasmSdkErrorKind::DriveInternalError, msg, None, retriable)
            }
            NoAvailableAddressesToRetry(inner) => Self::new(
                WasmSdkErrorKind::DapiClientError,
                format!("no available addresses to retry, last error: {}", inner),
                None,
                retriable,
            ),
        }
    }
}
impl From<ProtocolError> for WasmSdkError {
    fn from(err: ProtocolError) -> Self {
        match err {
            ProtocolError::ConsensusError(error) => {
                Self::protocol_with_consensus_errors(vec![*error])
            }
            ProtocolError::ConsensusErrors(errors) => Self::protocol_with_consensus_errors(errors),
            other => Self::new(WasmSdkErrorKind::Protocol, other.to_string(), None, false),
        }
    }
}

impl WasmSdkError {
    fn protocol_consensus_error_message<T: std::fmt::Display>(errors: &[T]) -> Option<String> {
        errors.first().map(ToString::to_string)
    }

    fn protocol_with_consensus_errors(
        errors: Vec<dash_sdk::dpp::consensus::ConsensusError>,
    ) -> Self {
        if errors.is_empty() {
            let details = Object::new();
            let _ = Reflect::set(
                &details,
                &JsValue::from_str("type"),
                &JsValue::from_str("ConsensusErrors"),
            );
            let _ = Reflect::set(
                &details,
                &JsValue::from_str("messages"),
                &Array::new().into(),
            );
            let _ = Reflect::set(&details, &JsValue::from_str("errors"), &Array::new().into());

            let mut error = Self::new(
                WasmSdkErrorKind::Protocol,
                "Protocol error contained an empty consensus error list",
                None,
                false,
            );
            error.details = details.into();
            return error;
        }

        let details = Object::new();
        let messages = Array::new();
        let structured_errors = Array::new();

        for error in &errors {
            messages.push(&JsValue::from_str(&error.to_string()));

            let structured_error = Object::new();
            let _ = Reflect::set(
                &structured_error,
                &JsValue::from_str("message"),
                &JsValue::from_str(&error.to_string()),
            );
            let _ = Reflect::set(
                &structured_error,
                &JsValue::from_str("code"),
                &JsValue::from_f64(error.code() as f64),
            );
            structured_errors.push(&structured_error.into());
        }

        let kind = if errors.len() == 1 {
            "ConsensusError"
        } else {
            "ConsensusErrors"
        };
        let message = Self::protocol_consensus_error_message(&errors)
            .expect("consensus errors should be non-empty after the empty-list guard");

        let _ = Reflect::set(
            &details,
            &JsValue::from_str("type"),
            &JsValue::from_str(kind),
        );
        let _ = Reflect::set(&details, &JsValue::from_str("messages"), &messages.into());
        let _ = Reflect::set(
            &details,
            &JsValue::from_str("errors"),
            &structured_errors.into(),
        );

        Self {
            kind: WasmSdkErrorKind::Protocol,
            message,
            code: -1,
            is_retriable: false,
            details: details.into(),
        }
    }
}

impl From<StateTransitionBroadcastError> for WasmSdkError {
    fn from(err: StateTransitionBroadcastError) -> Self {
        Self::new(
            WasmSdkErrorKind::StateTransitionBroadcastError,
            err.to_string(),
            Some(err.code as i32),
            false,
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
        Self::new(kind, err.to_string(), None, false)
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
            K::ExecutionNotProved => "ExecutionNotProved",
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

    /// Optional machine-readable details for JS callers.
    #[wasm_bindgen(getter)]
    pub fn details(&self) -> JsValue {
        self.details.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::WasmSdkError;

    #[test]
    fn protocol_consensus_error_message_uses_first_error() {
        let errors = ["first error", "second error"];

        assert_eq!(
            WasmSdkError::protocol_consensus_error_message(&errors),
            Some("first error".to_string())
        );
    }

    #[test]
    fn protocol_consensus_error_message_is_none_for_empty_input() {
        let errors: [&str; 0] = [];

        assert_eq!(
            WasmSdkError::protocol_consensus_error_message(&errors),
            None
        );
    }
}
