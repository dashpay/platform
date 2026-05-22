use dash_sdk::dpp::consensus::{codes::ErrorWithCode, ConsensusError};
use dash_sdk::dpp::ProtocolError;
use dash_sdk::{error::StateTransitionBroadcastError, Error as SdkError};
use js_sys::{Array, Object, Reflect};
use rs_dapi_client::CanRetry;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
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
    /// Structured carrier for protocol consensus errors.
    consensus_errors: Vec<WasmConsensusError>,
}

// wasm-bindgen getters defined below in the second impl block

#[derive(Debug, Clone, Eq, PartialEq)]
struct WasmConsensusError {
    kind: String,
    name: String,
    message: String,
    code: u32,
}

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
            consensus_errors: Vec::new(),
        }
    }

    fn from_protocol_error(err: ProtocolError, is_retriable: bool) -> Self {
        let (message, code, consensus_errors) = match &err {
            ProtocolError::ConsensusError(error) => {
                let consensus = error.as_ref();
                (
                    consensus.to_string(),
                    consensus.code() as i32,
                    vec![WasmConsensusError::from_consensus_error(consensus)],
                )
            }
            ProtocolError::ConsensusErrors(errors) => {
                let message = errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                let consensus_errors = errors
                    .iter()
                    .map(WasmConsensusError::from_consensus_error)
                    .collect();
                (message, -1, consensus_errors)
            }
            _ => (err.to_string(), -1, Vec::new()),
        };

        Self {
            kind: WasmSdkErrorKind::Protocol,
            message,
            code,
            is_retriable,
            consensus_errors,
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
            Protocol(e) => Self::from_protocol_error(e, retriable),
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
        Self::from_protocol_error(err, false)
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
        // Map WasmDppError kind to appropriate WasmSdkError kind. If wasm-dpp2
        // preserved protocol consensus errors, thread them through to the same
        // structured JS `consensusErrors` surface as direct ProtocolError paths.
        let kind = match err.kind() {
            WasmDppErrorKind::Protocol => WasmSdkErrorKind::Protocol,
            WasmDppErrorKind::InvalidArgument => WasmSdkErrorKind::InvalidArgument,
            WasmDppErrorKind::Serialization => WasmSdkErrorKind::SerializationError,
            WasmDppErrorKind::Conversion => WasmSdkErrorKind::SerializationError,
            WasmDppErrorKind::Generic => WasmSdkErrorKind::Generic,
        };
        let consensus_errors = err
            .consensus_errors()
            .iter()
            .map(WasmConsensusError::from_consensus_error)
            .collect::<Vec<_>>();
        if consensus_errors.is_empty() {
            Self::new(kind, err.to_string(), None, false)
        } else {
            Self {
                kind,
                message: err.to_string(),
                code: err.code(),
                is_retriable: false,
                consensus_errors,
            }
        }
    }
}

fn consensus_error_kind_name(err: &ConsensusError) -> &'static str {
    match err {
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
/// variant is added or renamed. Mirrors `consensus_error_variant_name` in
/// `rs-sdk-ffi`.
fn consensus_error_variant_name(err: &ConsensusError) -> &'static str {
    match err {
        ConsensusError::DefaultError => "DefaultError",
        ConsensusError::BasicError(inner) => inner.into(),
        ConsensusError::StateError(inner) => inner.into(),
        ConsensusError::SignatureError(inner) => inner.into(),
        ConsensusError::FeeError(inner) => inner.into(),
    }
}

impl WasmConsensusError {
    fn from_consensus_error(err: &ConsensusError) -> Self {
        Self {
            kind: consensus_error_kind_name(err).to_string(),
            name: consensus_error_variant_name(err).to_string(),
            message: err.to_string(),
            code: err.code(),
        }
    }

    fn to_js_value(&self) -> JsValue {
        let object = Object::new();
        let _ = Reflect::set(
            &object,
            &JsValue::from_str("kind"),
            &JsValue::from_str(&self.kind),
        );
        let _ = Reflect::set(
            &object,
            &JsValue::from_str("name"),
            &JsValue::from_str(&self.name),
        );
        let _ = Reflect::set(
            &object,
            &JsValue::from_str("message"),
            &JsValue::from_str(&self.message),
        );
        let _ = Reflect::set(
            &object,
            &JsValue::from_str("code"),
            &JsValue::from_f64(self.code as f64),
        );
        object.into()
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

    /// Structured protocol consensus errors when the originating protocol error
    /// was `ProtocolError::ConsensusError` or `ProtocolError::ConsensusErrors`.
    #[wasm_bindgen(
        getter = "consensusErrors",
        unchecked_return_type = "Array<{ kind: string; name: string; message: string; code: number }> | undefined"
    )]
    pub fn consensus_errors(&self) -> JsValue {
        if self.consensus_errors.is_empty() {
            return JsValue::UNDEFINED;
        }

        Array::from_iter(
            self.consensus_errors
                .iter()
                .map(WasmConsensusError::to_js_value),
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::consensus::basic::document::DocumentTransitionsAreAbsentError;

    #[test]
    fn protocol_consensus_errors_plural_are_preserved_structurally() {
        let absent_err: ConsensusError = DocumentTransitionsAreAbsentError::new().into();
        let absent_message = absent_err.to_string();
        let error = WasmSdkError::from(ProtocolError::ConsensusErrors(vec![
            absent_err,
            ConsensusError::DefaultError,
        ]));

        assert_eq!(error.kind, WasmSdkErrorKind::Protocol);
        assert!(!error.is_retriable);
        assert_eq!(error.consensus_errors.len(), 2);
        assert_eq!(error.consensus_errors[0].kind, "BasicError");
        assert_eq!(
            error.consensus_errors[0].name,
            "DocumentTransitionsAreAbsentError"
        );
        assert_eq!(error.consensus_errors[0].message, absent_message);
        assert_eq!(error.consensus_errors[1].kind, "DefaultError");
        assert_eq!(error.consensus_errors[1].name, "DefaultError");
        assert_eq!(error.consensus_errors[1].code, 1);
        assert_eq!(error.code, -1);

        // Plural messages are joined with "; " for readability,
        // mirroring the rs-sdk-ffi behavior.
        let expected_message = format!("{}; {}", absent_message, ConsensusError::DefaultError);
        assert_eq!(error.message, expected_message);
        assert!(!error.message.contains("Multiple consensus errors: ["));
    }

    #[test]
    fn protocol_consensus_errors_singular_is_preserved_structurally() {
        let consensus_error: ConsensusError = DocumentTransitionsAreAbsentError::new().into();
        let expected_message = consensus_error.to_string();
        let expected_code = consensus_error.code();
        let error = WasmSdkError::from(ProtocolError::ConsensusError(Box::new(consensus_error)));

        assert_eq!(error.kind, WasmSdkErrorKind::Protocol);
        assert!(!error.is_retriable);
        assert_eq!(error.consensus_errors.len(), 1);
        assert_eq!(error.consensus_errors[0].kind, "BasicError");
        assert_eq!(
            error.consensus_errors[0].name,
            "DocumentTransitionsAreAbsentError"
        );
        assert_eq!(error.consensus_errors[0].message, expected_message);
        assert_eq!(error.consensus_errors[0].code, expected_code);
        assert_eq!(error.code, expected_code as i32);
        // Singular keeps the inner consensus error's Display unchanged.
        assert_eq!(error.message, expected_message);
    }

    #[test]
    fn sdk_protocol_errors_use_protocol_mapping() {
        let sdk_error = SdkError::Protocol(ProtocolError::ConsensusErrors(vec![
            ConsensusError::DefaultError,
            ConsensusError::DefaultError,
        ]));
        let retriable = sdk_error.can_retry();
        let error = WasmSdkError::from(sdk_error);

        assert_eq!(error.kind, WasmSdkErrorKind::Protocol);
        assert_eq!(error.is_retriable, retriable);
        assert_eq!(error.consensus_errors.len(), 2);
        assert!(error.message.contains("; "));
    }

    #[test]
    fn wasm_dpp_error_consensus_errors_are_preserved_structurally() {
        let dpp_error = WasmDppError::from(ProtocolError::ConsensusErrors(vec![
            ConsensusError::DefaultError,
            ConsensusError::DefaultError,
        ]));
        let error = WasmSdkError::from(dpp_error);

        assert_eq!(error.kind, WasmSdkErrorKind::Protocol);
        assert_eq!(error.code, -1);
        assert_eq!(error.consensus_errors.len(), 2);
        assert_eq!(error.consensus_errors[0].name, "DefaultError");
        assert!(error.message.contains("; "));
    }
}
