use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
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

    /// GroveDB error, often for issues with proofs
    #[error("grovedb: {error}")]
    GroveDBError {
        proof_bytes: Vec<u8>,
        height: u64,
        time_ms: u64,
        error: String,
    },

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
}

/// Errors returned by the context provider
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
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

pub(crate) trait MapGroveDbError<O> {
    fn map_drive_error(self, proof: &Proof, metadata: &ResponseMetadata) -> Result<O, Error>;
}

impl<O> MapGroveDbError<O> for Result<O, drive::error::Error> {
    fn map_drive_error(self, proof: &Proof, metadata: &ResponseMetadata) -> Result<O, Error> {
        match self {
            Ok(o) => Ok(o),
            Err(e) => match e {
                drive::error::Error::GroveDB(e) => Err(Error::GroveDBError {
                    proof_bytes: proof.grovedb_proof.clone(),
                    height: metadata.height,
                    time_ms: metadata.time_ms,
                    error: e.to_string(),
                }),
                _ => Err(e.into()),
            },
        }
    }
}
