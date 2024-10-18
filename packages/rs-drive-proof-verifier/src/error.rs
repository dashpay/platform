use dpp::ProtocolError;

/// Errors
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    /// Not initialized
    #[error("not initialized: call initialize() first")]
    NotInitialized,

    #[error("already initialized: initialize() can be called only once")]
    AlreadyInitialized,

    /// Drive error
    #[error("dash drive: {error}")]
    DriveError { error: String },

    /// Dash Protocol error
    #[error("dash protocol: {error}")]
    ProtocolError { error: String },

    /// Empty response metadata
    #[error("empty response metadata")]
    EmptyResponseMetadata,

    /// Empty version
    #[error("empty version")]
    EmptyVersion,

    /// No proof in response
    #[error("no proof in result")]
    NoProofInResult,

    /// Requested object not found
    #[error("requested object not found")]
    NotFound,

    /// Decode protobuf error
    #[error("decode request: {error}")]
    RequestError { error: String },

    /// Decode protobuf response error
    #[error("decode response: {error}")]
    ResponseDecodeError { error: String },

    /// Error when preparing result
    #[error("result encoding: {error}")]
    ResultEncodingError { error: String },

    /// Cannot generate signature digest for data
    #[error("cannot generate signature digest for data: {error}")]
    SignDigestFailed { error: String },

    /// Error during signature verification
    #[error("error during signature verification: {error}")]
    SignatureVerificationError { error: String },

    /// Signature format is invalid
    #[error("invalid signature format: {error}")]
    InvalidSignatureFormat { error: String },

    /// Public key is invalid
    #[error("invalid public key: {error}")]
    InvalidPublicKey { error: String },

    /// Invalid signature
    #[error("invalid signature: {error}")]
    InvalidSignature { error: String },

    /// Callback error
    #[error("unexpected callback error: {error}, reason: {reason}")]
    UnexpectedCallbackError { error: String, reason: String },

    /// Invalid version of object in response
    #[error("invalid version of message")]
    InvalidVersion(#[from] dpp::version::PlatformVersionError),

    /// Context provider is not set
    #[error("context provider is not set")]
    ContextProviderNotSet,

    /// Context provider error
    #[error("context provider error: {0}")]
    ContextProviderError(#[from] ContextProviderError),

    /// Proof is stale; try another server
    #[error("proof is stale; try another server")]
    StaleProof(#[from] StaleProofError),
}

/// Received proof is stale; try another server
#[derive(Debug, thiserror::Error)]
pub enum StaleProofError {
    /// Stale proof height
    #[error("stale proof height: expected height {expected_height}, received {actual_height}, tolerance {tolerance}, try another server")]
    StaleProofHeight {
        /// Expected height - last block height seen by the Sdk
        expected_height: u64,
        /// Actual height - block height received from the server in the proof
        actual_height: u64,
        /// Tolerance - how many blocks can be behind the expected height
        tolerance: u64,
    },
    /// Proof time is stale
    #[error(
        "received outdated proof time: expected {expected_ms} ms, received {actual_ms} ms, tolerance {tolerance_ms} ms, try another server"
    )]
    Time {
        /// Expected time in milliseconds - is local time when the proof was received
        expected_ms: u64,
        /// Actual time in milliseconds - time received from the server in the proof
        actual_ms: u64,
        /// Tolerance in milliseconds
        tolerance_ms: u64,
    },
}

/// Errors returned by the context provider
#[derive(Debug, thiserror::Error)]
pub enum ContextProviderError {
    /// Generic Context provider error
    #[error("Context provider error: {0}")]
    Generic(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Data contract is invalid or not found, or some error occurred during data contract retrieval
    #[error("cannot get data contract: {0}")]
    DataContractFailure(String),

    /// Provided quorum is invalid
    #[error("invalid quorum: {0}")]
    InvalidQuorum(String),

    /// Core Fork Error
    #[error("activation fork error: {0}")]
    ActivationForkError(String),

    /// Async error, eg. when tokio runtime fails
    #[error("async error: {0}")]
    AsyncError(String),
}

impl From<drive::error::Error> for Error {
    fn from(error: drive::error::Error) -> Self {
        Self::DriveError {
            error: error.to_string(),
        }
    }
}

impl From<ProtocolError> for Error {
    fn from(error: ProtocolError) -> Self {
        Self::ProtocolError {
            error: error.to_string(),
        }
    }
}
