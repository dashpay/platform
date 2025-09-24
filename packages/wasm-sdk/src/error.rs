use dash_sdk::dpp::ProtocolError;
use dash_sdk::{error::StateTransitionBroadcastError, Error as SdkError};
use rs_dapi_client::CanRetry;
use wasm_bindgen::prelude::wasm_bindgen;

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
    Generic,
    ContextProviderError,
    Cancelled,
    StaleNode,
    StateTransitionBroadcastError,

    // Local helper kinds
    InvalidArgument,
    SerializationError,
    NotFound,
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
    retriable: bool,
}

// wasm-bindgen getters defined below in the second impl block

impl WasmSdkError {
    fn new<M: Into<String>>(
        kind: WasmSdkErrorKind,
        message: M,
        code: Option<i32>,
        retriable: bool,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            code: code.unwrap_or(-1),
            retriable,
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
            Protocol(e) => Self::new(WasmSdkErrorKind::Protocol, e.to_string(), None, retriable),
            Proof(e) => Self::new(WasmSdkErrorKind::Proof, e.to_string(), None, retriable),
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
            DapiMocksError(e) => Self::new(
                WasmSdkErrorKind::DapiMocksError,
                e.to_string(),
                None,
                retriable,
            ),
        }
    }
}
impl From<ProtocolError> for WasmSdkError {
    fn from(err: ProtocolError) -> Self {
        Self::new(WasmSdkErrorKind::Protocol, err.to_string(), None, false)
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
            K::Generic => "Generic",
            K::ContextProviderError => "ContextProviderError",
            K::Cancelled => "Cancelled",
            K::StaleNode => "StaleNode",
            K::StateTransitionBroadcastError => "StateTransitionBroadcastError",
            K::InvalidArgument => "InvalidArgument",
            K::SerializationError => "SerializationError",
            K::NotFound => "NotFound",
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
    #[wasm_bindgen(getter)]
    pub fn retriable(&self) -> bool {
        self.retriable
    }
}
