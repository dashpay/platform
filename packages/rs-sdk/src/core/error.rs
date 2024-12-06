//! Errors that can occur in the Dash Core.

use drive_proof_verifier::error::ContextProviderError;
use rs_dapi_client::CanRetry;

/// Dash Core still warming up
pub const CORE_RPC_ERROR_IN_WARMUP: i32 = -28;
/// Dash Core Client is not connected
pub const CORE_RPC_CLIENT_NOT_CONNECTED: i32 = -9;
/// Dash Core still downloading initial blocks
pub const CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD: i32 = -10;

#[derive(Debug, thiserror::Error)]
/// Errors that can occur when communicating with the Dash Core.
pub enum DashCoreError {
    /// Error from Dash Core.
    #[error("Dash Core RPC error: {0}")]
    Rpc(#[from] dashcore_rpc::Error),
    /// Quorum is invalid.
    #[error("Invalid quorum: {0}")]
    InvalidQuorum(String),

    /// Fork activation error - most likely the fork is not activated yet.
    #[error("Fork activation: {0}")]
    ActivationForkError(String),
}

impl From<DashCoreError> for ContextProviderError {
    fn from(error: DashCoreError) -> Self {
        match error {
            DashCoreError::Rpc(e) => Self::DashCoreError(e.to_string()),
            DashCoreError::InvalidQuorum(e) => Self::InvalidQuorum(e),
            DashCoreError::ActivationForkError(e) => Self::ActivationForkError(e),
        }
    }
}

impl CanRetry for DashCoreError {
    fn can_retry(&self) -> bool {
        use dashcore_rpc::jsonrpc::error::Error as JsonRpcError;
        use dashcore_rpc::Error as RpcError;
        match self {
            DashCoreError::Rpc(RpcError::JsonRpc(JsonRpcError::Transport(..))) => true,
            DashCoreError::Rpc(RpcError::JsonRpc(JsonRpcError::Rpc(e))) => {
                matches!(
                    e.code,
                    CORE_RPC_ERROR_IN_WARMUP
                        | CORE_RPC_CLIENT_NOT_CONNECTED
                        | CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD,
                )
            }
            _ => false,
        }
    }
}
