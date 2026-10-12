use dash_sdk::dpp::ProtocolError;
use dash_sdk::platform::encrypted_for::EncryptedForError;
use dash_sdk::{error::StateTransitionBroadcastError, Error as SdkError};
use rs_dapi_client::CanRetry;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_dpp2::error::{consensus_error_code, WasmDppError};

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
    /// An `encryptedFor` property did not decrypt: the keys are not the ones it was encrypted
    /// with, or the bytes are corrupt.
    DecryptionFailed,
    /// No key of an identity can serve as the recipient or sender key of an `encryptedFor`
    /// property: none meets the schema's `keyRequirements`, or the private key given is not
    /// one of the identity's keys.
    EncryptionKeyNotFound,
}

/// Structured error surfaced to JS consumers
#[wasm_bindgen]
#[derive(thiserror::Error, Debug, Clone)]
#[error("{message}")]
pub struct WasmSdkError {
    kind: WasmSdkErrorKind,
    message: String,
    /// The consensus error code (`10422`, `40132`, ...) when the error is a consensus error,
    /// whether Platform refused the transition or the SDK caught it before broadcast; `-1`
    /// otherwise.
    code: i32,
    /// Indicates if the operation can be retried safely.
    is_retriable: bool,
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
        }
    }

    /// Converts `err` as `?` would, keeping its kind, consensus code and whether it can be
    /// retried, under the message `"{context}: {err}"`.
    pub(crate) fn with_context<E>(context: &str, err: E) -> Self
    where
        E: std::fmt::Display + Into<Self>,
    {
        let message = format!("{context}: {err}");
        Self {
            message,
            ..err.into()
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
            Protocol(e) => Self::new(
                WasmSdkErrorKind::Protocol,
                e.to_string(),
                consensus_error_code(&e),
                retriable,
            ),
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
            EncryptedFor(e) => e.into(),
        }
    }
}

impl From<EncryptedForError> for WasmSdkError {
    fn from(err: EncryptedForError) -> Self {
        let kind = match err {
            EncryptedForError::DecryptionFailed => WasmSdkErrorKind::DecryptionFailed,
            EncryptedForError::NoSuitableKey { .. } => WasmSdkErrorKind::EncryptionKeyNotFound,
            _ => WasmSdkErrorKind::InvalidArgument,
        };
        Self::new(kind, err.to_string(), None, false)
    }
}
impl From<ProtocolError> for WasmSdkError {
    fn from(err: ProtocolError) -> Self {
        let code = consensus_error_code(&err);
        Self::new(WasmSdkErrorKind::Protocol, err.to_string(), code, false)
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
        Self {
            kind,
            message: err.to_string(),
            code: err.code(),
            is_retriable: false,
        }
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
            K::DecryptionFailed => "DecryptionFailed",
            K::EncryptionKeyNotFound => "EncryptionKeyNotFound",
        }
        .to_string()
    }

    /// Human-readable message
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// The consensus error code when the error is a consensus error (compare it with
    /// `DocumentPropertyConstraintErrorCode` and the other code enums); -1 otherwise.
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> i32 {
        self.code
    }

    /// Whether the error is retryable
    #[wasm_bindgen(getter = "isRetriable")]
    pub fn is_retriable(&self) -> bool {
        self.is_retriable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::tonic::metadata::{MetadataMap, MetadataValue};
    use dapi_grpc::tonic::{Code, Status};
    use dash_sdk::dpp::consensus::basic::document::{
        DocumentPropertyConstraintViolatedError, PropertyConstraintViolation,
    };
    use dash_sdk::dpp::consensus::state::contract_moderation::ContractUserBannedError;
    use dash_sdk::dpp::consensus::ConsensusError;
    use dash_sdk::dpp::data_contract::errors::DataContractError;
    use dash_sdk::dpp::platform_value::Identifier;
    use dash_sdk::dpp::serialization::PlatformSerializableWithPlatformVersion;
    use dash_sdk::dpp::version::PlatformVersion;
    use rs_dapi_client::transport::TransportError;
    use rs_dapi_client::DapiClientError;

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

    /// What the SDK makes of DAPI refusing a transition at CheckTx: a gRPC status carrying the
    /// serialized consensus error in its metadata.
    fn refused_by_platform(consensus_error: &ConsensusError) -> SdkError {
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
        SdkError::from(DapiClientError::Transport(TransportError::Grpc(status)))
    }

    #[test]
    fn should_carry_the_code_of_a_consensus_error_platform_refused_the_transition_with() {
        let error = WasmSdkError::from(refused_by_platform(&property_constraint_violated()));

        assert_eq!(error.kind(), WasmSdkErrorKind::Protocol);
        assert_eq!(error.code(), 10422);
    }

    #[test]
    fn should_carry_the_consensus_code_under_the_message_of_the_operation_that_failed() {
        let error = WasmSdkError::with_context(
            "Failed to mint tokens",
            refused_by_platform(&contract_user_banned()),
        );

        assert_eq!(error.kind(), WasmSdkErrorKind::Protocol);
        assert_eq!(error.code(), 41107);
        assert!(!error.is_retriable());
        assert_eq!(
            error.message(),
            format!(
                "Failed to mint tokens: Protocol error: {}",
                contract_user_banned()
            )
        );
    }

    #[test]
    fn should_carry_the_code_of_a_consensus_error_the_sdk_caught_before_broadcast() {
        let error = WasmSdkError::from(ProtocolError::from(property_constraint_violated()));

        assert_eq!(error.kind(), WasmSdkErrorKind::Protocol);
        assert_eq!(error.code(), 10422);
    }

    #[test]
    fn should_carry_the_code_of_a_consensus_error_through_a_wasm_dpp_error() {
        let dpp_error = WasmDppError::from(ProtocolError::from(contract_user_banned()));
        let error = WasmSdkError::from(dpp_error);

        assert_eq!(error.kind(), WasmSdkErrorKind::Protocol);
        assert_eq!(error.code(), 41107);
    }

    #[test]
    fn should_carry_the_code_platform_refuses_a_contract_error_with() {
        let contract_error = DataContractError::InvalidContractStructure("malformed".to_string());
        let error = WasmSdkError::from(WasmDppError::from(ProtocolError::from(contract_error)));

        assert_eq!(error.code(), 10231);
    }

    #[test]
    fn should_report_no_code_for_a_protocol_error_that_is_not_a_consensus_error() {
        let error = WasmSdkError::from(ProtocolError::Generic("not consensus".to_string()));

        assert_eq!(error.code(), -1);
    }

    #[test]
    fn should_keep_the_broadcast_error_code() {
        let error = WasmSdkError::with_context(
            "Failed to broadcast",
            SdkError::from(StateTransitionBroadcastError {
                code: 40132,
                message: "fee agreement not set".to_string(),
                cause: None,
            }),
        );

        assert_eq!(
            error.kind(),
            WasmSdkErrorKind::StateTransitionBroadcastError
        );
        assert_eq!(error.code(), 40132);
    }
}
