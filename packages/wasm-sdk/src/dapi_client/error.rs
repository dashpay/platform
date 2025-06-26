//! Error types for DAPI client

use thiserror::Error;

/// DAPI client errors
#[derive(Error, Debug)]
pub enum DapiClientError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Response error: {0}")]
    Response(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),

    #[error("All endpoints failed")]
    AllEndpointsFailed,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Protocol error: {0}")]
    Protocol(String),
}