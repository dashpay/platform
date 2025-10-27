use std::io;

use ciborium::de::Error as CborError;
use thiserror::Error;
use tokio::time::error::Elapsed;

pub type CliResult<T> = Result<T, CliError>;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid DAPI URL '{url}': {source}")]
    InvalidUrl {
        url: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("failed to connect to DAPI service: {0}")]
    Transport(#[from] tonic::transport::Error),
    #[error(transparent)]
    Status(#[from] tonic::Status),
    #[error("invalid state transition hash '{hash}': {source}")]
    InvalidHash {
        hash: String,
        #[source]
        source: hex::FromHexError,
    },
    #[error("invalid state transition payload: {0}")]
    InvalidStateTransition(#[from] hex::FromHexError),
    #[error(transparent)]
    Timeout(#[from] Elapsed),
    #[error("CBOR decode error: {0}")]
    Cbor(#[from] CborError<io::Error>),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("received empty response from {0}")]
    EmptyResponse(&'static str),

    #[error(transparent)]
    DashCoreEncoding(#[from] dashcore::consensus::encode::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}
