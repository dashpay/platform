use dpp::platform_value::Error as ValueError;
use dpp::ProtocolError;
use drive::error::proof::ProofError;
use drive::error::query::QuerySyntaxError as SyntaxError;
use drive::error::Error as DriveError;
use prost::DecodeError;

/// Errors
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    /// Proof Error
    #[error("proof error: {0}")]
    Proof(#[from] ProofError),
    /// Syntax Error
    #[error("query syntax error: {0}")]
    Query(#[from] SyntaxError),

    /// Protocol Error
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Value Error
    #[error("query value error: {0}")]
    Value(#[from] ValueError),

    /// Drive Error
    #[error("drive error: {0}")]
    Drive(#[from] DriveError),

    /// Decoding error Error
    #[error("protobuf decoding error: {0}")]
    ProtobufDecode(#[from] DecodeError),
}
