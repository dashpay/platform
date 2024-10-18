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

    /// Remote node is stale; try another server
    #[error(transparent)]
    StaleNode(#[from] StaleNodeError),
}

/// Server returned stale metadata
#[derive(Debug, thiserror::Error)]
pub enum StaleNodeError {
    /// Server returned metadata with outdated height
    #[error("received height is outdated: expected {expected_height}, received {received_height}, tolerance {tolerance_blocks}; try another server")]
    Height {
        /// Expected height - last block height seen by the Sdk
        expected_height: u64,
        /// Block height received from the server
        received_height: u64,
        /// Tolerance - how many blocks can be behind the expected height
        tolerance_blocks: u64,
    },
    /// Server returned metadata with time outside of the tolerance
    #[error(
        "received invalid time: expected {expected_timestamp_ms}ms, received {received_timestamp_ms} ms, tolerance {tolerance_ms} ms; try another server"
    )]
    Time {
        /// Expected time in milliseconds - is local time when the message was received
        expected_timestamp_ms: u64,
        /// Time received from the server in the message, in milliseconds
        received_timestamp_ms: u64,
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
