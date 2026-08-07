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
    /// Input to a document builder failed validation (bad label, wrong
    /// ciphertext length, unknown document type, ...). `dash-sdk` maps this
    /// to its `Error::Generic`, preserving the messages these checks
    /// produced before they moved here.
    #[error("{0}")]
    InvalidInput(String),
    /// Drive error
    #[error("Drive error: {0}")]
    Drive(#[from] drive::error::Error),
    /// DPP error
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}

impl From<crate::documents::proto_conversions::DecodeError> for Error {
    fn from(value: crate::documents::proto_conversions::DecodeError) -> Self {
        use crate::documents::proto_conversions::DecodeError;
        match value {
            // Malformed wire bytes — a decoding failure, not a
            // misconfiguration.
            DecodeError::InvalidArgument(msg) => Self::Protocol(ProtocolError::DecodingError(msg)),
            // Well-formed wire shape the decode target can't express
            // yet — same classification the server gives it.
            DecodeError::Unsupported(msg) => Self::Drive(drive::error::Error::Query(
                drive::error::query::QuerySyntaxError::Unsupported(msg),
            )),
        }
    }
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
