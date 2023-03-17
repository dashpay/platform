use crate::abci::Error as AbciError;
use crate::error::execution::ExecutionError;
use crate::error::serialization::SerializationError;
use drive::dpp::ProtocolError;
use drive::error::Error as DriveError;
use tracing::error;
/// Execution errors module
pub mod execution;

/// Serialization errors module
pub mod serialization;

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
    /// Serialization Error
    #[error("serialization: {0}")]
    Serialization(#[from] SerializationError),
    /// Configuration Error
    #[error("configuration: {0}")]
    Configuration(#[from] envy::Error),
}
