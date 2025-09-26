// Custom error types for rs-dapi using thiserror

use base64::{Engine, engine};
use dapi_grpc::platform::v0::StateTransitionBroadcastError;
use dashcore_rpc::{self, jsonrpc};
use dpp::{consensus::ConsensusError, serialization::PlatformDeserializable};
use serde_json::Value;
use sha2::Digest;
use std::{fmt, os::linux::raw};
use thiserror::Error;
use tokio::task::JoinError;
use tonic::{Code, metadata::MetadataValue};

/// Result type alias for DAPI operations
pub type DapiResult<T> = std::result::Result<T, DapiError>;

/// Main error type for DAPI operations
#[derive(Error, Debug)]
pub enum DapiError {
    #[error("ZMQ connection error: {0}")]
    ZmqConnection(#[from] zeromq::ZmqError),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Streaming service error: {0}")]
    StreamingService(String),

    #[error("Client error: {0}")]
    Client(String),

    #[error("Cannot connect to server {0}: {1}")]
    /// Server unavailable error (URI, detailed message)
    ServerUnavailable(String, String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("Status error: {0}")]
    Status(#[from] tonic::Status),

    #[error("HTTP error: {0}")]
    Http(#[from] axum::http::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] Box<tokio_tungstenite::tungstenite::Error>),

    #[error("Task join error: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("Transaction hash not found in event attributes")]
    TransactionHashNotFound,

    #[error("Invalid data: {0}")]
    InvalidData(String),

    // Standardized categories for RPC-like errors
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Aborted: {0}")]
    Aborted(String),

    #[error("Unavailable: {0}")]
    Unavailable(String),

    #[error("Failed precondition: {0}")]
    FailedPrecondition(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Client is gone: {0}")]
    ClientGone(String),

    #[error("No valid proof found for tx: {0}")]
    NoValidTxProof(String),

    #[error("{0}")]
    MethodNotFound(String),

    #[error("Tenderdash request error: {0:?}")]
    TenderdashClientError(TenderdashBroadcastError),
}

/// Result type alias for DAPI operations
pub type DAPIResult<T> = std::result::Result<T, DapiError>;

// Add From implementation for boxed errors
impl From<Box<dyn std::error::Error + Send + Sync>> for DapiError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for DapiError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(err))
    }
}

impl From<DapiError> for tonic::Status {
    fn from(err: DapiError) -> Self {
        err.to_status()
    }
}

impl DapiError {
    /// Create a tonic::Status from DapiError.
    ///
    /// Defaults to internal status if status cannot be converted.
    pub fn to_status(&self) -> tonic::Status {
        match self {
            DapiError::Status(status) => status.clone(),
            DapiError::NotFound(msg) => tonic::Status::not_found(msg.clone()),
            DapiError::AlreadyExists(msg) => tonic::Status::already_exists(msg.clone()),
            DapiError::InvalidArgument(msg) => tonic::Status::invalid_argument(msg.clone()),
            DapiError::ResourceExhausted(msg) => tonic::Status::resource_exhausted(msg.clone()),
            DapiError::Aborted(msg) => tonic::Status::aborted(msg.clone()),
            DapiError::Unavailable(msg) | DapiError::ServiceUnavailable(msg) => {
                tonic::Status::unavailable(msg.clone())
            }
            DapiError::FailedPrecondition(msg) => tonic::Status::failed_precondition(msg.clone()),
            DapiError::MethodNotFound(msg) => tonic::Status::unimplemented(msg.clone()),
            DapiError::TenderdashClientError(error) => error.to_status(),
            _ => tonic::Status::internal(self.to_string()),
        }
    }

    pub fn from_tenderdash_error(value: Value) -> Self {
        DapiError::TenderdashClientError(TenderdashBroadcastError::from(value))
    }

    /// Create a no proof error for a transaction
    pub fn no_valid_tx_proof(tx: &[u8]) -> Self {
        let tx_hash = if tx.len() == sha2::Sha256::output_size() {
            // possible false positive if tx is not a hash but still a 32-byte array
            hex::encode(tx)
        } else {
            let digest = sha2::Sha256::digest(tx);
            hex::encode(digest)
        };
        Self::NoValidTxProof(tx_hash)
    }

    /// Create a configuration error
    pub fn configuration<S: Into<String>>(msg: S) -> Self {
        Self::Configuration(msg.into())
    }

    /// Create a streaming service error
    pub fn streaming_service<S: Into<String>>(msg: S) -> Self {
        Self::StreamingService(msg.into())
    }

    /// Create a client error
    pub fn client<S: Into<String>>(msg: S) -> Self {
        Self::Client(msg.into())
    }

    /// Convert this error into a tonic::Status while preserving legacy codes/messages when available.
    pub fn into_legacy_status(self) -> tonic::Status {
        match self {
            DapiError::NotFound(msg) => tonic::Status::new(tonic::Code::NotFound, msg),
            DapiError::AlreadyExists(msg) => tonic::Status::new(tonic::Code::AlreadyExists, msg),
            DapiError::InvalidArgument(msg) => {
                tonic::Status::new(tonic::Code::InvalidArgument, msg)
            }
            DapiError::ResourceExhausted(msg) => {
                tonic::Status::new(tonic::Code::ResourceExhausted, msg)
            }
            DapiError::FailedPrecondition(msg) => {
                tonic::Status::new(tonic::Code::FailedPrecondition, msg)
            }
            DapiError::Client(msg) => tonic::Status::new(tonic::Code::InvalidArgument, msg),
            DapiError::ServiceUnavailable(msg) | DapiError::Unavailable(msg) => {
                tonic::Status::new(tonic::Code::Unavailable, msg)
            }
            DapiError::MethodNotFound(msg) => tonic::Status::new(tonic::Code::Unimplemented, msg),
            DapiError::Timeout(msg) => tonic::Status::new(tonic::Code::DeadlineExceeded, msg),
            DapiError::Aborted(msg) => tonic::Status::new(tonic::Code::Aborted, msg),
            other => other.to_status(),
        }
    }

    /// Create a connection validation error
    pub fn server_unavailable<U: ToString, S: ToString>(uri: U, msg: S) -> Self {
        Self::ServerUnavailable(uri.to_string(), msg.to_string())
    }

    /// Create a server error
    pub fn server<S: Into<String>>(msg: S) -> Self {
        Self::Server(msg.into())
    }

    /// Create an invalid data error
    pub fn invalid_data<S: Into<String>>(msg: S) -> Self {
        Self::InvalidData(msg.into())
    }

    /// Create a service unavailable error
    pub fn service_unavailable<S: Into<String>>(msg: S) -> Self {
        Self::ServiceUnavailable(msg.into())
    }

    /// Create a timeout error
    pub fn timeout<S: Into<String>>(msg: S) -> Self {
        Self::Timeout(msg.into())
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Self::Internal(msg.into())
    }

    /// Handle task join errors
    pub fn map_join_result<T: Sized, E: Into<Self>>(
        msg: Result<Result<T, E>, JoinError>,
    ) -> Result<T, Self> {
        match msg {
            Ok(Ok(inner)) => Ok(inner),
            Ok(Err(e)) => Err(e.into()),
            Err(join_err) => Err(DapiError::TaskJoin(join_err)),
        }
    }
}

#[derive(Clone)]
pub struct TenderdashBroadcastError {
    pub code: i64,
    // human-readable error message; will be put into `data` field
    pub message: Option<String>,
    // CBOR-encoded dpp ConsensusError
    pub consensus_error: Option<Vec<u8>>,
}

impl TenderdashBroadcastError {
    pub fn to_status(&self) -> tonic::Status {
        let status_code = self.grpc_code();
        let status_message = self.grpc_message();

        let mut status: tonic::Status = tonic::Status::new(status_code, status_message);

        if let Some(consensus_error) = &self.consensus_error {
            // Add consensus error metadata
            status.metadata_mut().insert_bin(
                "dash-serialized-consensus-error-bin",
                MetadataValue::from_bytes(consensus_error),
            );
        }
        status
    }

    fn grpc_message(&self) -> String {
        if let Some(message) = &self.message {
            return message.clone();
        }

        if let Some(consensus_error_bytes) = &self.consensus_error
            && let Ok(consensus_error) =
                ConsensusError::deserialize_from_bytes(&consensus_error_bytes).inspect_err(|e| {
                    tracing::warn!("Failed to deserialize consensus error: {}", e);
                })
        {
            return consensus_error.to_string();
        }

        return format!("Unknown error with code {}", self.code);
    }

    /// map gRPC code from Tenderdash to tonic::Code.
    ///
    /// See packages/rs-dpp/src/errors/consensus/codes.rs for possible codes.
    fn grpc_code(&self) -> Code {
        match self.code {
            0 => Code::Ok,
            1 => Code::Cancelled,
            2 => Code::Unknown,
            3 => Code::InvalidArgument,
            4 => Code::DeadlineExceeded,
            5 => Code::NotFound,
            6 => Code::AlreadyExists,
            7 => Code::PermissionDenied,
            8 => Code::ResourceExhausted,
            9 => Code::FailedPrecondition,
            10 => Code::Aborted,
            11 => Code::OutOfRange,
            12 => Code::Unimplemented,
            13 => Code::Internal,
            14 => Code::Unavailable,
            15 => Code::DataLoss,
            16 => Code::Unauthenticated,
            code => {
                if (17..=9999).contains(&code) {
                    Code::Unknown
                } else if (10000..20000).contains(&code) {
                    Code::InvalidArgument
                } else if (20000..30000).contains(&code) {
                    Code::Unauthenticated
                } else if (30000..40000).contains(&code) {
                    Code::FailedPrecondition
                } else if (40000..50000).contains(&code) {
                    Code::InvalidArgument
                } else {
                    Code::Internal
                }
            }
        }
    }
}

impl From<TenderdashBroadcastError> for StateTransitionBroadcastError {
    fn from(err: TenderdashBroadcastError) -> Self {
        StateTransitionBroadcastError {
            code: err.code.min(u32::MAX as i64) as u32,
            message: err.message.unwrap_or_else(|| "Unknown error".to_string()),
            data: err.consensus_error.unwrap_or_default(),
        }
    }
}

impl fmt::Debug for TenderdashBroadcastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TenderdashBroadcastError")
            .field("code", &self.code)
            .field("message", &self.message)
            .field(
                "consensus_error",
                &self
                    .consensus_error
                    .as_ref()
                    .map(|e| hex::encode(e))
                    .unwrap_or_else(|| "None".to_string()),
            )
            .finish()
    }
}

pub(crate) fn base64_decode(input: &str) -> Option<Vec<u8>> {
    static BASE64: engine::GeneralPurpose = {
        let b64_config = engine::GeneralPurposeConfig::new()
            .with_decode_allow_trailing_bits(true)
            .with_encode_padding(false)
            .with_decode_padding_mode(engine::DecodePaddingMode::Indifferent);

        engine::GeneralPurpose::new(&base64::alphabet::STANDARD, b64_config)
    };
    BASE64
        .decode(input)
        .inspect_err(|e| {
            tracing::warn!("Failed to decode base64: {}", e);
        })
        .ok()
}

fn decode_drive_error_info(info_base64: String) -> Option<Vec<u8>> {
    let decoded_bytes = base64_decode(&info_base64)?;
    // CBOR-decode decoded_bytes
    let raw_value: Value = ciborium::de::from_reader(decoded_bytes.as_slice())
        .inspect_err(|e| {
            tracing::warn!("Failed to decode drive error info from CBOR: {}", e);
        })
        .ok()?;

    let data_map = raw_value
        .get("data")
        .and_then(|d| d.as_object())
        .or_else(|| {
            tracing::trace!("Drive error info missing 'data' field");
            None
        })?;

    let serialized_error = data_map
        .get("serializedError")
        .or_else(|| data_map.get("serialized_error"))
        .and_then(|se| se.as_array())
        .or_else(|| {
            tracing::trace!("Drive error info missing 'serializedError' field");
            None
        })?;

    // convert serialized_error from array of numbers to Vec<u8>
    let serialized_error: Vec<u8> = serialized_error
        .iter()
        .filter_map(|v| {
            v.as_u64()
                .and_then(|n| if n <= 255 { Some(n as u8) } else { None })
                .or_else(|| {
                    tracing::warn!(
                        "Drive error info 'serializedError' contains non-u8 value: {:?}",
                        v
                    );
                    None
                })
        })
        .collect();

    Some(serialized_error)
}

impl From<Value> for TenderdashBroadcastError {
    // Convert from a JSON error object returned by Tenderdash RPC, typically in the `error` field of a JSON-RPC response.
    fn from(value: Value) -> Self {
        if let Some(object) = value.as_object() {
            let code = object
                .get("code")
                .and_then(|c| c.as_i64())
                .unwrap_or_else(|| {
                    tracing::debug!("Tenderdash error missing 'code' field, defaulting to 0");
                    0
                });
            let message = object
                .get("message")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            // info contains additional error details, possibly including consensus error
            let consensus_error = object
                .get("info")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .and_then(decode_drive_error_info);

            Self {
                code,
                message,
                consensus_error,
            }
        } else {
            tracing::warn!("Tenderdash error is not an object: {:?}", value);
            Self {
                code: u32::MAX as i64,
                message: Some("Invalid error object from Tenderdash".to_string()),
                consensus_error: None,
            }
        }
    }
}

pub trait MapToDapiResult<T: Sized> {
    fn to_dapi_result(self) -> DAPIResult<T>;
}

impl<T: Sized, E: Into<DapiError>> MapToDapiResult<T> for Result<Result<T, E>, JoinError> {
    fn to_dapi_result(self) -> DAPIResult<T> {
        match self {
            Ok(Ok(inner)) => Ok(inner),
            Ok(Err(e)) => Err(e.into()),
            Err(e) => Err(e.into()),
        }
    }
}

impl<T: Sized> MapToDapiResult<T> for DapiResult<T> {
    fn to_dapi_result(self) -> DAPIResult<T> {
        self
    }
}

// Provide a conversion from dashcore-rpc Error to our DapiError so callers can
// use generic helpers like MapToDapiResult without custom closures.
impl From<dashcore_rpc::Error> for DapiError {
    fn from(e: dashcore_rpc::Error) -> Self {
        match e {
            dashcore_rpc::Error::JsonRpc(jerr) => match jerr {
                jsonrpc::Error::Rpc(rpc) => {
                    let code = rpc.code;
                    let msg = rpc.message;
                    match code {
                        -5 => DapiError::NotFound(msg), // Invalid address or key / Not found
                        -8 => DapiError::NotFound(msg), // Block height out of range
                        -1 => DapiError::InvalidArgument(msg), // Invalid parameter
                        -27 => DapiError::AlreadyExists(msg), // Already in chain
                        -26 => DapiError::FailedPrecondition(msg), // RPC_VERIFY_REJECTED
                        -25 | -22 => DapiError::InvalidArgument(msg), // Deserialization/Verify error
                        _ => DapiError::Unavailable(format!("Core RPC error {}: {}", code, msg)),
                    }
                }
                jsonrpc::Error::Transport(_) => DapiError::Unavailable(jerr.to_string()),
                jsonrpc::Error::Json(_) => DapiError::InvalidData(jerr.to_string()),
                _ => DapiError::Unavailable(jerr.to_string()),
            },
            dashcore_rpc::Error::BitcoinSerialization(e) => DapiError::InvalidData(e.to_string()),
            dashcore_rpc::Error::Hex(e) => DapiError::InvalidData(e.to_string()),
            dashcore_rpc::Error::Json(e) => DapiError::InvalidData(e.to_string()),
            dashcore_rpc::Error::Io(e) => DapiError::Io(e),
            dashcore_rpc::Error::InvalidAmount(e) => DapiError::InvalidData(e.to_string()),
            dashcore_rpc::Error::Secp256k1(e) => DapiError::InvalidData(e.to_string()),
            dashcore_rpc::Error::InvalidCookieFile => {
                DapiError::Unavailable("invalid cookie file".to_string())
            }
            dashcore_rpc::Error::UnexpectedStructure(s) => DapiError::InvalidData(s),
        }
    }
}
