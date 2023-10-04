use std::fmt::Debug;

use dpp::ProtocolError;
use rs_dapi_client::DapiClientError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid configuration: {0}")]
    Config(String),
    #[error("Drive error: {0}")]
    Drive(#[from] drive::error::Error),
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("Proof verification error: {0}")]
    Proof(#[from] drive_proof_verifier::Error),
    #[error("Dapi client error: {0}")]
    DapiClientError(String),
    #[error("Core client error: {0}")]
    CoreClientError(#[from] dashcore_rpc::Error),
    #[error("Invalid url: {0}")]
    InvalidUrl(#[from] http::uri::InvalidUri),
    #[error("Object not found: {0}")]
    NotFound(String),
    #[error("Too many elements received, expected: {expected}, got: {got}")]
    TooManyElementsReceived { expected: usize, got: usize },
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
}

impl<T: Debug> From<DapiClientError<T>> for Error {
    fn from(value: DapiClientError<T>) -> Self {
        Self::DapiClientError(format!("{:?}", value))
    }
}
