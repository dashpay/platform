// Custom error types for rs-dapi using thiserror

use serde_json::Value;
use sha2::Digest;
use std::fmt;
use thiserror::Error;
// For converting dashcore-rpc errors into DapiError
use crate::services::platform_service::map_drive_code_to_status;
use dashcore_rpc::{self, jsonrpc};
use tokio::task::JoinError;

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

    #[error("Tenderdash request error: {0}")]
    TenderdashRestError(TenderdashRpcError),
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
            DapiError::TenderdashRestError(error) => error.to_status(),
            _ => tonic::Status::internal(self.to_string()),
        }
    }

    pub fn from_tenderdash_error(value: Value) -> Self {
        DapiError::TenderdashRestError(TenderdashRpcError::from(value))
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

#[derive(Debug, Clone)]
pub struct TenderdashRpcError {
    pub code: Option<i64>,
    pub message: Option<String>,
    pub data: Option<Value>,
}

impl TenderdashRpcError {
    pub fn data_as_str(&self) -> Option<&str> {
        self.data.as_ref()?.as_str()
    }

    pub fn to_status(&self) -> tonic::Status {
        if let Some(code) = self.code {
            let info = self.data_as_str();
            return map_drive_code_to_status(code, info);
        }

        let message = self
            .message
            .clone()
            .or_else(|| self.data_as_str().map(str::to_owned))
            .unwrap_or_else(|| "Unknown Tenderdash error".to_string());

        tonic::Status::internal(message)
    }
}

impl fmt::Display for TenderdashRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.code, &self.message, &self.data) {
            (Some(code), Some(message), _) => write!(f, "code {code}: {message}"),
            (Some(code), None, Some(data)) => write!(f, "code {code}: {data}"),
            (Some(code), None, None) => write!(f, "code {code}"),
            (None, Some(message), _) => f.write_str(message),
            (_, _, Some(data)) => write!(f, "{data}"),
            _ => f.write_str("unknown"),
        }
    }
}

impl From<Value> for TenderdashRpcError {
    fn from(value: Value) -> Self {
        if let Some(object) = value.as_object() {
            let code = object.get("code").and_then(|c| c.as_i64());
            let message = object
                .get("message")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());
            let data = object.get("data").cloned();

            Self {
                code,
                message,
                data,
            }
        } else {
            Self {
                code: None,
                message: None,
                data: Some(value),
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
