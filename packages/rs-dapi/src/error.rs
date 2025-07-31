// Custom error types for rs-dapi using thiserror

use thiserror::Error;

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
    WebSocket(String),

    #[error("Task join error: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for DAPI operations
pub type DAPIResult<T> = std::result::Result<T, DapiError>;

// Add From implementation for boxed errors
impl From<Box<dyn std::error::Error + Send + Sync>> for DapiError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Internal(err.to_string())
    }
}

impl DapiError {
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

    /// Create a server error
    pub fn server<S: Into<String>>(msg: S) -> Self {
        Self::Server(msg.into())
    }

    /// Create a WebSocket error
    pub fn websocket<S: Into<String>>(msg: S) -> Self {
        Self::WebSocket(msg.into())
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
}
