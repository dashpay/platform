// Custom error types for rs-dapi using thiserror

use dashcore_rpc::{self, jsonrpc};
use serde_json::Value;
use sha2::Digest;
use thiserror::Error;
use tokio::task::JoinError;

use crate::services::platform_service::TenderdashStatus;

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

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

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
    TenderdashClientError(TenderdashStatus),
}

/// Result type alias for DAPI operations
pub type DAPIResult<T> = std::result::Result<T, DapiError>;

// Add From implementation for boxed errors
impl From<Box<dyn std::error::Error + Send + Sync>> for DapiError {
    /// Collapse boxed dynamic errors into an internal error variant.
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for DapiError {
    /// Wrap tungstenite errors in the WebSocket variant.
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(err))
    }
}

impl From<DapiError> for tonic::Status {
    /// Convert `DapiError` directly into `tonic::Status` using `to_status`.
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

    /// Construct a `DapiError` from a raw Tenderdash JSON status response.
    pub fn from_tenderdash_error(value: Value) -> Self {
        DapiError::TenderdashClientError(TenderdashStatus::from(value))
    }

    /// Create a no proof error for a transaction.
    ///
    /// Note that this assumes that if tx is 32 bytes, it is already a hash.
    /// If the input has a different size, it will be hashed.
    /// It can lead to false positives if a non-hash 32-byte array is passed.
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

pub trait MapToDapiResult<T: Sized> {
    /// Convert nested or join results into `DAPIResult<T>` for convenience APIs.
    fn to_dapi_result(self) -> DAPIResult<T>;
}

impl<T: Sized, E: Into<DapiError>> MapToDapiResult<T> for Result<Result<T, E>, JoinError> {
    /// Flatten `Result<Result<..>>` from spawned tasks into `DAPIResult`.
    fn to_dapi_result(self) -> DAPIResult<T> {
        match self {
            Ok(Ok(inner)) => Ok(inner),
            Ok(Err(e)) => Err(e.into()),
            Err(e) => Err(e.into()),
        }
    }
}

impl<T: Sized, E: Into<DapiError>> MapToDapiResult<T> for Result<T, E> {
    /// Flatten `Result<Result<..>>` from spawned tasks into `DAPIResult`.
    fn to_dapi_result(self) -> DAPIResult<T> {
        match self {
            Ok(inner) => Ok(inner),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use serde_json::json;

    // -- to_status tests --

    #[test]
    fn to_status_maps_not_found() {
        let err = DapiError::NotFound("missing item".into());
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::NotFound);
        assert_eq!(status.message(), "missing item");
    }

    #[test]
    fn to_status_maps_already_exists() {
        let err = DapiError::AlreadyExists("dup".into());
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::AlreadyExists);
        assert_eq!(status.message(), "dup");
    }

    #[test]
    fn to_status_maps_invalid_argument() {
        let err = DapiError::InvalidArgument("bad arg".into());
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert_eq!(status.message(), "bad arg");
    }

    #[test]
    fn to_status_maps_resource_exhausted() {
        let err = DapiError::ResourceExhausted("too many".into());
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::ResourceExhausted);
        assert_eq!(status.message(), "too many");
    }

    #[test]
    fn to_status_maps_aborted() {
        let err = DapiError::Aborted("cancelled".into());
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::Aborted);
        assert_eq!(status.message(), "cancelled");
    }

    #[test]
    fn to_status_maps_unavailable() {
        let err = DapiError::Unavailable("down".into());
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::Unavailable);
        assert_eq!(status.message(), "down");
    }

    #[test]
    fn to_status_maps_service_unavailable() {
        let err = DapiError::ServiceUnavailable("offline".into());
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::Unavailable);
        assert_eq!(status.message(), "offline");
    }

    #[test]
    fn to_status_maps_failed_precondition() {
        let err = DapiError::FailedPrecondition("precond".into());
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);
        assert_eq!(status.message(), "precond");
    }

    #[test]
    fn to_status_maps_method_not_found() {
        let err = DapiError::MethodNotFound("no such method".into());
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
        assert_eq!(status.message(), "no such method");
    }

    #[test]
    fn to_status_maps_status_passthrough() {
        let original = tonic::Status::permission_denied("forbidden");
        let err = DapiError::Status(original);
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::PermissionDenied);
        assert_eq!(status.message(), "forbidden");
    }

    #[test]
    fn to_status_defaults_to_internal_for_other_variants() {
        let err = DapiError::Configuration("bad config".into());
        let status = err.to_status();
        assert_eq!(status.code(), tonic::Code::Internal);
        assert!(status.message().contains("bad config"));

        let err2 = DapiError::ConnectionClosed;
        let status2 = err2.to_status();
        assert_eq!(status2.code(), tonic::Code::Internal);
    }

    // -- From<DapiError> for tonic::Status --

    #[test]
    fn dapi_error_converts_to_tonic_status() {
        let err = DapiError::NotFound("gone".into());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    // -- From<Box<dyn Error>> --

    #[test]
    fn boxed_error_converts_to_internal() {
        let boxed: Box<dyn std::error::Error + Send + Sync> = "dynamic error".into();
        let err: DapiError = boxed.into();
        match err {
            DapiError::Internal(msg) => assert_eq!(msg, "dynamic error"),
            other => panic!("expected Internal, got {:?}", other),
        }
    }

    // -- into_legacy_status tests --

    #[test]
    fn into_legacy_status_maps_not_found() {
        let err = DapiError::NotFound("x".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::NotFound);
        assert_eq!(status.message(), "x");
    }

    #[test]
    fn into_legacy_status_maps_already_exists() {
        let err = DapiError::AlreadyExists("dup".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::AlreadyExists);
    }

    #[test]
    fn into_legacy_status_maps_invalid_argument() {
        let err = DapiError::InvalidArgument("bad".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn into_legacy_status_maps_resource_exhausted() {
        let err = DapiError::ResourceExhausted("full".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::ResourceExhausted);
    }

    #[test]
    fn into_legacy_status_maps_failed_precondition() {
        let err = DapiError::FailedPrecondition("precond".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);
    }

    #[test]
    fn into_legacy_status_maps_client_to_invalid_argument() {
        let err = DapiError::Client("client err".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert_eq!(status.message(), "client err");
    }

    #[test]
    fn into_legacy_status_maps_service_unavailable() {
        let err = DapiError::ServiceUnavailable("down".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::Unavailable);
    }

    #[test]
    fn into_legacy_status_maps_unavailable() {
        let err = DapiError::Unavailable("gone".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::Unavailable);
    }

    #[test]
    fn into_legacy_status_maps_method_not_found() {
        let err = DapiError::MethodNotFound("nope".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[test]
    fn into_legacy_status_maps_timeout() {
        let err = DapiError::Timeout("timed out".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::DeadlineExceeded);
    }

    #[test]
    fn into_legacy_status_maps_aborted() {
        let err = DapiError::Aborted("abort".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::Aborted);
    }

    #[test]
    fn into_legacy_status_falls_through_to_to_status() {
        let err = DapiError::Internal("boom".into());
        let status = err.into_legacy_status();
        assert_eq!(status.code(), tonic::Code::Internal);
    }

    // -- no_valid_tx_proof tests --

    #[test]
    fn no_valid_tx_proof_with_32_bytes_uses_hex_directly() {
        let hash = [0xab_u8; 32];
        let err = DapiError::no_valid_tx_proof(&hash);
        match err {
            DapiError::NoValidTxProof(hex_str) => {
                assert_eq!(hex_str, hex::encode(hash));
            }
            other => panic!("expected NoValidTxProof, got {:?}", other),
        }
    }

    #[test]
    fn no_valid_tx_proof_with_non_32_bytes_hashes_input() {
        let tx = b"hello tx";
        let err = DapiError::no_valid_tx_proof(tx);
        match err {
            DapiError::NoValidTxProof(hex_str) => {
                let expected = hex::encode(sha2::Sha256::digest(tx));
                assert_eq!(hex_str, expected);
            }
            other => panic!("expected NoValidTxProof, got {:?}", other),
        }
    }

    // -- from_tenderdash_error --

    #[test]
    fn from_tenderdash_error_creates_tenderdash_client_error() {
        let value = json!({"code": 42, "message": "test error"});
        let err = DapiError::from_tenderdash_error(value);
        match err {
            DapiError::TenderdashClientError(status) => {
                assert_eq!(status.code, 42);
            }
            other => panic!("expected TenderdashClientError, got {:?}", other),
        }
    }

    // -- constructor helpers --

    #[test]
    fn constructor_configuration() {
        match DapiError::configuration("cfg err") {
            DapiError::Configuration(msg) => assert_eq!(msg, "cfg err"),
            other => panic!("expected Configuration, got {:?}", other),
        }
    }

    #[test]
    fn constructor_streaming_service() {
        match DapiError::streaming_service("stream err") {
            DapiError::StreamingService(msg) => assert_eq!(msg, "stream err"),
            other => panic!("expected StreamingService, got {:?}", other),
        }
    }

    #[test]
    fn constructor_client() {
        match DapiError::client("client err") {
            DapiError::Client(msg) => assert_eq!(msg, "client err"),
            other => panic!("expected Client, got {:?}", other),
        }
    }

    #[test]
    fn constructor_server() {
        match DapiError::server("server err") {
            DapiError::Server(msg) => assert_eq!(msg, "server err"),
            other => panic!("expected Server, got {:?}", other),
        }
    }

    #[test]
    fn constructor_server_unavailable() {
        match DapiError::server_unavailable("http://foo", "conn refused") {
            DapiError::ServerUnavailable(uri, msg) => {
                assert_eq!(uri, "http://foo");
                assert_eq!(msg, "conn refused");
            }
            other => panic!("expected ServerUnavailable, got {:?}", other),
        }
    }

    // -- map_join_result --

    #[tokio::test]
    async fn map_join_result_ok_ok() {
        let handle = tokio::spawn(async { Ok::<_, DapiError>(42) });
        let result = DapiError::map_join_result(handle.await);
        assert_eq!(result.expect("map_join_result should succeed"), 42);
    }

    #[tokio::test]
    async fn map_join_result_ok_err() {
        let handle =
            tokio::spawn(async { Err::<i32, DapiError>(DapiError::Internal("inner".into())) });
        let result = DapiError::map_join_result(handle.await);
        assert!(matches!(result.unwrap_err(), DapiError::Internal(_)));
    }

    #[tokio::test]
    async fn map_join_result_join_error() {
        let handle = tokio::spawn(async {
            panic!("deliberate panic for test");
            #[allow(unreachable_code)]
            Ok::<i32, DapiError>(0)
        });
        let result = DapiError::map_join_result(handle.await);
        assert!(matches!(result.unwrap_err(), DapiError::TaskJoin(_)));
    }

    // -- MapToDapiResult tests --

    #[tokio::test]
    async fn map_to_dapi_result_nested_ok() {
        let handle = tokio::spawn(async { Ok::<_, DapiError>(99) });
        let result: DAPIResult<i32> = handle.await.to_dapi_result();
        assert_eq!(result.expect("nested ok should unwrap"), 99);
    }

    #[tokio::test]
    async fn map_to_dapi_result_nested_inner_err() {
        let handle =
            tokio::spawn(async { Err::<i32, DapiError>(DapiError::NotFound("nope".into())) });
        let result: DAPIResult<i32> = handle.await.to_dapi_result();
        assert!(matches!(result.unwrap_err(), DapiError::NotFound(_)));
    }

    #[test]
    fn map_to_dapi_result_flat_ok() {
        let r: Result<i32, DapiError> = Ok(10);
        let result: DAPIResult<i32> = r.to_dapi_result();
        assert_eq!(result.expect("flat ok should unwrap"), 10);
    }

    #[test]
    fn map_to_dapi_result_flat_err() {
        let r: Result<i32, DapiError> = Err(DapiError::Timeout("too slow".into()));
        let result: DAPIResult<i32> = r.to_dapi_result();
        assert!(matches!(result.unwrap_err(), DapiError::Timeout(_)));
    }

    // -- From<dashcore_rpc::Error> --

    #[test]
    fn from_dashcore_rpc_not_found_code_minus_5() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -5,
            message: "not found".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::NotFound(msg) if msg == "not found"));
    }

    #[test]
    fn from_dashcore_rpc_invalid_argument_code_minus_8() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -8,
            message: "out of range".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::InvalidArgument(msg) if msg == "out of range"));
    }

    #[test]
    fn from_dashcore_rpc_internal_code_minus_1() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -1,
            message: "misc error".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::Internal(msg) if msg == "misc error"));
    }

    #[test]
    fn from_dashcore_rpc_invalid_argument_code_minus_3() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -3,
            message: "type error".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::InvalidArgument(msg) if msg == "type error"));
    }

    #[test]
    fn from_dashcore_rpc_not_found_code_minus_18() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -18,
            message: "wallet not found".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::NotFound(msg) if msg == "wallet not found"));
    }

    #[test]
    fn from_dashcore_rpc_unavailable_code_minus_28() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -28,
            message: "still warming up".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::Unavailable(msg) if msg == "still warming up"));
    }

    #[test]
    fn from_dashcore_rpc_already_exists_code_minus_27() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -27,
            message: "already in chain".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::AlreadyExists(msg) if msg == "already in chain"));
    }

    #[test]
    fn from_dashcore_rpc_failed_precondition_code_minus_26() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -26,
            message: "rejected".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::FailedPrecondition(msg) if msg == "rejected"));
    }

    #[test]
    fn from_dashcore_rpc_failed_precondition_code_minus_25() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -25,
            message: "verify error".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::FailedPrecondition(msg) if msg == "verify error"));
    }

    #[test]
    fn from_dashcore_rpc_invalid_argument_code_minus_22() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -22,
            message: "deser err".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::InvalidArgument(msg) if msg == "deser err"));
    }

    #[test]
    fn from_dashcore_rpc_unknown_code_maps_to_unavailable() {
        let rpc_err = dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(jsonrpc::error::RpcError {
            code: -999,
            message: "unknown".to_string(),
            data: None,
        }));
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::Unavailable(msg) if msg.contains("-999")));
    }

    #[test]
    fn from_dashcore_rpc_invalid_cookie_file() {
        let rpc_err = dashcore_rpc::Error::InvalidCookieFile;
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::Unavailable(msg) if msg.contains("cookie")));
    }

    #[test]
    fn from_dashcore_rpc_unexpected_structure() {
        let rpc_err = dashcore_rpc::Error::UnexpectedStructure("bad struct".to_string());
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::InvalidData(msg) if msg == "bad struct"));
    }

    #[test]
    fn from_dashcore_rpc_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broken");
        let rpc_err = dashcore_rpc::Error::Io(io_err);
        let err: DapiError = rpc_err.into();
        assert!(matches!(err, DapiError::Io(_)));
    }
}

// Provide a conversion from dashcore-rpc Error to our DapiError so callers can
// use generic helpers like MapToDapiResult without custom closures.
impl From<dashcore_rpc::Error> for DapiError {
    /// Map dashcore RPC errors into rich `DapiError` variants for uniform handling.
    fn from(e: dashcore_rpc::Error) -> Self {
        match e {
            dashcore_rpc::Error::JsonRpc(jerr) => match jerr {
                jsonrpc::Error::Rpc(rpc) => {
                    let code = rpc.code;
                    let msg = rpc.message;
                    match code {
                        // RPC_INVALID_ADDRESS_OR_KEY: address/key not found
                        -5 => DapiError::NotFound(msg),
                        // RPC_TYPE_ERROR: unexpected type for a parameter
                        -3 => DapiError::InvalidArgument(msg),
                        // RPC_INVALID_PARAMETER: invalid, missing or duplicate parameter
                        -8 => DapiError::InvalidArgument(msg),
                        // RPC_MISC_ERROR: std::exception thrown in command handling
                        -1 => DapiError::Internal(msg),
                        // RPC_WALLET_NOT_FOUND
                        -18 => DapiError::NotFound(msg),
                        // RPC_TRANSACTION_ALREADY_IN_CHAIN
                        -27 => DapiError::AlreadyExists(msg),
                        // RPC_VERIFY_REJECTED: transaction or block rejected
                        -26 => DapiError::FailedPrecondition(msg),
                        // RPC_VERIFY_ERROR: general verification error
                        -25 => DapiError::FailedPrecondition(msg),
                        // RPC_DESERIALIZATION_ERROR: raw tx/block deserialization
                        -22 => DapiError::InvalidArgument(msg),
                        // RPC_IN_WARMUP: node is still starting up
                        -28 => DapiError::Unavailable(msg),
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
