use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::serialization::SerializationError;
use crate::logging;
use dashcore_rpc::Error as CoreRpcError;
use drive::dpp::ProtocolError;
use drive::error::Error as DriveError;
use tenderdash_abci::proto::abci::ResponseException;
use tracing::error;

/// Execution errors module
pub mod execution;

/// Query errors module
pub mod query;
/// Serialization errors module
pub mod serialization;

// @append_only
/// Errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// ABCI Server Error
    #[error("abci: {0}")]
    Abci(#[from] AbciError),
    /// Drive Error
    #[error("storage: {0}")]
    Drive(#[from] DriveError),
    /// Protocol Error
    #[error("protocol: {0}")]
    Protocol(#[from] ProtocolError),
    /// Execution Error
    #[error("execution: {0}")]
    Execution(#[from] ExecutionError),
    /// Core RPC Error
    #[error("core rpc error: {0}")]
    CoreRpc(#[from] CoreRpcError),
    /// Serialization Error
    #[error("serialization: {0}")]
    Serialization(#[from] SerializationError),
    /// Configuration Error
    #[error("configuration: {0}")]
    Configuration(#[from] envy::Error),
    /// Logging error
    #[error("logging: {0}")]
    Logging(#[from] logging::Error),
    /// Error from metrics subsystem
    #[error("metrics: {0}")]
    Metrics(#[from] crate::metrics::Error),
}

impl From<Error> for ResponseException {
    fn from(value: Error) -> Self {
        Self {
            error: value.to_string(),
        }
    }
}
