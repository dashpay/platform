//! Definitions of errors
use std::fmt::Debug;

use dpp::ProtocolError;
use rs_dapi_client::DapiClientError;

/// Error type for the SDK
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// SDK is not configured properly
    #[error("SDK misconfigured: {0}")]
    Config(String),
    /// Drive error
    #[error("Drive error: {0}")]
    Drive(#[from] drive::error::Error),
    /// DPP error
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    /// Proof verification error
    #[error("Proof verification error: {0}")]
    Proof(#[from] drive_proof_verifier::Error),
    /// DAPI client error, for example, connection error
    #[error("Dapi client error: {0}")]
    DapiClientError(String),
    /// Core client error, for example, connection error
    #[error("Core client error: {0}")]
    CoreClientError(#[from] dashcore_rpc::Error),
    /// Dependency not found, for example data contract for a document not found
    #[error("Required {0} not found: {1}")]
    MissingDependency(String, String),
    /// Epoch not found; we must have at least one epoch
    #[error("No epoch found on the Platform; it should never happen")]
    EpochNotFound,
}

impl<T: Debug> From<DapiClientError<T>> for Error {
    fn from(value: DapiClientError<T>) -> Self {
        Self::DapiClientError(format!("{:?}", value))
    }
}
