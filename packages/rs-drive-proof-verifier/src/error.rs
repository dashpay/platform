use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::ProtocolError;
use drive::grovedb::Error as GroveError;
use drive::query::PathQuery;

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
        path_query: Option<PathQuery>,
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
    ContextProviderError(#[from] dash_context_provider::ContextProviderError),
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

            Err(drive::error::Error::GroveDB(grove_err)) => {
                // If InvalidProof error is returned, extract the path query from it
                let maybe_query = match grove_err.as_ref() {
                    GroveError::InvalidProof(path_query, ..) => Some(path_query.clone()),
                    _ => None,
                };

                Err(Error::GroveDBError {
                    proof_bytes: proof.grovedb_proof.clone(),
                    path_query: maybe_query,
                    height: metadata.height,
                    time_ms: metadata.time_ms,
                    error: grove_err.to_string(),
                })
            }

            Err(other) => Err(other.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drive::grovedb::{Error as GroveError, Query, SizedQuery};

    /// Helper: build a minimal `Proof` with the given grovedb_proof bytes.
    fn test_proof(grovedb_proof: Vec<u8>) -> Proof {
        Proof {
            grovedb_proof,
            quorum_hash: vec![0u8; 32],
            signature: vec![0u8; 96],
            round: 1,
            block_id_hash: vec![0u8; 32],
            quorum_type: 1,
        }
    }

    /// Helper: build a minimal `ResponseMetadata`.
    fn test_metadata(height: u64, time_ms: u64) -> ResponseMetadata {
        ResponseMetadata {
            height,
            core_chain_locked_height: 1,
            epoch: 0,
            time_ms,
            protocol_version: 1,
            chain_id: "test-chain".to_string(),
        }
    }

    /// Helper: build a simple `PathQuery`.
    fn test_path_query() -> PathQuery {
        let query = Query::new();
        let sized = SizedQuery::new(query, Some(10), None);
        PathQuery::new(vec![vec![1, 2, 3]], sized)
    }

    #[test]
    fn test_map_drive_error_ok_passes_through() {
        let result: Result<u64, drive::error::Error> = Ok(42u64);
        let proof = test_proof(vec![0xAA, 0xBB]);
        let metadata = test_metadata(100, 5000);

        let mapped = result.map_drive_error(&proof, &metadata);
        assert_eq!(mapped.unwrap(), 42u64);
    }

    #[test]
    fn test_map_drive_error_grovedb_invalid_proof() {
        let pq = test_path_query();
        let grove_err = GroveError::InvalidProof(pq.clone(), "bad proof data".to_string());
        let drive_err = drive::error::Error::GroveDB(Box::new(grove_err));
        let result: Result<u64, drive::error::Error> = Err(drive_err);

        let proof_bytes = vec![0xDE, 0xAD];
        let proof = test_proof(proof_bytes.clone());
        let metadata = test_metadata(42, 9999);

        let mapped = result.map_drive_error(&proof, &metadata);
        let err = mapped.unwrap_err();

        match err {
            Error::GroveDBError {
                proof_bytes: pb,
                path_query: maybe_pq,
                height,
                time_ms,
                error,
            } => {
                assert_eq!(pb, proof_bytes, "proof_bytes should match the input proof");
                assert!(
                    maybe_pq.is_some(),
                    "path_query should be Some for InvalidProof"
                );
                assert_eq!(
                    maybe_pq.unwrap(),
                    pq,
                    "path_query should match the original"
                );
                assert_eq!(height, 42);
                assert_eq!(time_ms, 9999);
                assert!(
                    error.contains("bad proof data"),
                    "error message should contain the original message, got: {error}"
                );
            }
            other => panic!("expected GroveDBError, got: {other:?}"),
        }
    }

    #[test]
    fn test_map_drive_error_grovedb_other() {
        let grove_err = GroveError::InternalError("something broke".to_string());
        let drive_err = drive::error::Error::GroveDB(Box::new(grove_err));
        let result: Result<u64, drive::error::Error> = Err(drive_err);

        let proof = test_proof(vec![0x01]);
        let metadata = test_metadata(10, 2000);

        let mapped = result.map_drive_error(&proof, &metadata);
        let err = mapped.unwrap_err();

        match err {
            Error::GroveDBError {
                path_query, error, ..
            } => {
                assert!(
                    path_query.is_none(),
                    "path_query should be None for non-InvalidProof GroveDB errors"
                );
                assert!(
                    error.contains("something broke"),
                    "error message should be preserved, got: {error}"
                );
            }
            other => panic!("expected GroveDBError, got: {other:?}"),
        }
    }

    #[test]
    fn test_map_drive_error_non_grovedb() {
        let drive_err = drive::error::Error::Drive(
            drive::error::drive::DriveError::CorruptedCodeExecution("test drive error"),
        );
        let result: Result<u64, drive::error::Error> = Err(drive_err);

        let proof = test_proof(vec![]);
        let metadata = test_metadata(1, 1);

        let mapped = result.map_drive_error(&proof, &metadata);
        let err = mapped.unwrap_err();

        match err {
            Error::DriveError { error } => {
                assert!(
                    error.contains("test drive error"),
                    "error message should contain the original text, got: {error}"
                );
            }
            other => panic!("expected DriveError, got: {other:?}"),
        }
    }

    #[test]
    fn test_from_protocol_error() {
        let protocol_err = ProtocolError::IdentifierError("bad id".to_string());
        let err: Error = protocol_err.into();

        match err {
            Error::ProtocolError { error } => {
                assert!(
                    error.contains("bad id"),
                    "error message should contain the original text, got: {error}"
                );
            }
            other => panic!("expected ProtocolError, got: {other:?}"),
        }
    }

    #[test]
    fn test_from_drive_error() {
        let drive_err = drive::error::Error::Drive(
            drive::error::drive::DriveError::CorruptedCodeExecution("from conversion test"),
        );
        let err: Error = drive_err.into();

        match err {
            Error::DriveError { error } => {
                assert!(
                    error.contains("from conversion test"),
                    "error message should contain the original text, got: {error}"
                );
            }
            other => panic!("expected DriveError, got: {other:?}"),
        }
    }
}
