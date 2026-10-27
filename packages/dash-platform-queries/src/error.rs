//! Errors produced by the transport-free query core.

use dpp::consensus::ConsensusError;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::ProtocolError;

/// Error type for the transport-free query core.
///
/// `dash-sdk` converts this into its own `Error` via `From`, so code that
/// moved here from the SDK keeps working behind `?` at its old call sites.
// Same allowance rs-sdk's Error carries: ProtocolError dominates the size.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Query is not configured properly for the target platform version
    #[error("SDK misconfigured: {0}")]
    Config(String),
    /// Drive error
    #[error("Drive error: {0}")]
    Drive(#[from] drive::error::Error),
    /// DPP error
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}

impl From<ConsensusError> for Error {
    fn from(value: ConsensusError) -> Self {
        Self::Protocol(ProtocolError::ConsensusError(Box::new(value)))
    }
}

impl From<SimpleConsensusValidationResult> for Error {
    fn from(value: SimpleConsensusValidationResult) -> Self {
        value
            .errors
            .into_iter()
            .next()
            .map(Error::from)
            .unwrap_or_else(|| {
                Error::Protocol(ProtocolError::CorruptedCodeExecution(
                    "state transition structure validation failed without an error".to_string(),
                ))
            })
    }
}
