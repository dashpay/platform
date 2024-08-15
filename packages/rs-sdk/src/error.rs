//! Definitions of errors
use std::fmt::Debug;
use std::time::Duration;

use dapi_grpc::mock::Mockable;
use dpp::version::PlatformVersionError;
use dpp::ProtocolError;
use rs_dapi_client::{CanRetry, DapiClientError};

pub use drive_proof_verifier::error::ContextProviderError;

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
    /// Invalid Proved Response error
    #[error("Invalid Proved Response error: {0}")]
    InvalidProvedResponse(String),
    /// DAPI client error, for example, connection error
    #[error("Dapi client error: {0}")]
    DapiClientError(String),
    #[cfg(feature = "mocks")]
    /// DAPI mocks error
    #[error("Dapi mocks error: {0}")]
    DapiMocksError(#[from] rs_dapi_client::mock::MockError),
    /// Dash core error
    #[error("Dash core error: {0}")]
    CoreError(#[from] dpp::dashcore::Error),
    /// MerkleBlockError
    #[error("Dash core error: {0}")]
    MerkleBlockError(#[from] dpp::dashcore::merkle_tree::MerkleBlockError),
    /// Core client error, for example, connection error
    #[error("Core client error: {0}")]
    CoreClientError(#[from] dashcore_rpc::Error),
    /// Dependency not found, for example data contract for a document not found
    #[error("Required {0} not found: {1}")]
    MissingDependency(String, String),
    /// Epoch not found; we must have at least one epoch
    #[error("No epoch found on the Platform; it should never happen")]
    EpochNotFound,
    /// Quorum not found; try again later
    #[error(
        "Quorum {quorum_hash_hex} of type {quorum_type} at height {core_chain_locked_height}: {e}"
    )]
    QuorumNotFound {
        quorum_hash_hex: String,
        quorum_type: u32,
        core_chain_locked_height: u32,
        e: ContextProviderError,
    },

    /// Asset lock not found; try again later.
    ///
    /// ## Parameters
    ///
    /// - 0 - core locked height in asset lock
    /// - 1 - current core locked height on the platform
    #[error("Asset lock for core locked height {0} not available yet, max avaiable locked core height is {1}; try again later")]
    CoreLockedHeightNotYetAvailable(u32, u32),

    /// SDK operation timeout reached error
    #[error("SDK operation timeout {} secs reached: {1}", .0.as_secs())]
    TimeoutReached(Duration, String),
    /// Generic error
    // TODO: Use domain specific errors instead of generic ones
    #[error("SDK error: {0}")]
    Generic(String),

    /// Context provider error
    #[error("Context provider error: {0}")]
    ContextProviderError(#[from] ContextProviderError),

    /// Operation cancelled - cancel token was triggered, timeout, etc.
    #[error("Operation cancelled: {0}")]
    Cancelled(String),
}

impl<T: Debug + Mockable> From<DapiClientError<T>> for Error {
    fn from(value: DapiClientError<T>) -> Self {
        Self::DapiClientError(format!("{:?}", value))
    }
}

impl From<PlatformVersionError> for Error {
    fn from(value: PlatformVersionError) -> Self {
        Self::Protocol(value.into())
    }
}
impl CanRetry for Error {
    /// Returns true if the operation can be retried, false means it's unspecified
    /// False means
    fn can_retry(&self) -> bool {
        matches!(
            self,
            Error::CoreLockedHeightNotYetAvailable(_, _) | Error::QuorumNotFound { .. }
        )
    }
}
