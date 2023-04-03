use crate::rpc::core::CoreHeight;
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dashcore_rpc::json::QuorumHash;

/// Error returned by Core RPC endpoint
#[derive(Debug, thiserror::Error)]
pub enum ValidatorSetError {
    #[error{"Core RPC returned error: {0}"}]
    /// Error returned by RPC interface
    RpcError(#[from] dashcore_rpc::Error),

    /// Requested height is not found
    #[error{"No quorum of type {1:?} at core height {0:?} found"}]
    NoQuorumAtHeight(Option<CoreHeight>, QuorumType),

    /// Quorum with given hash not found
    #[error{"No quorum with hash {0:?} of type {1:?} found"}]
    QuorumNotFound(QuorumHash, QuorumType),

    /// Quorum with given hash not found
    #[error{"Invalid format of field {field}: {details}"}]
    InvalidDataFormat {
        /// the field that is invalid
        field: String,
        /// details explaining why the format is invalid
        details: String,
    },
}
