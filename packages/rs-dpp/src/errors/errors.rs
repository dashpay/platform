use crate::data_contract::errors::*;
use crate::document::errors::*;
use crate::mocks;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Identifier Error: {0}")]
    IdentifierError(String),
    #[error("String Decode Error {0}")]
    StringDecodeError(String),
    #[error("Public key data is not set")]
    EmptyPublicKeyDataError,
    #[error("Payload reached a {0}Kb limit")]
    MaxEncodedBytesReachedError(usize),
    #[error("Encoding Error - {0}")]
    EncodingError(String),
    #[error("Decoding Error - {0}")]
    DecodingError(String),
    #[error("Not included or invalid protocol version")]
    NoProtocolVersionError,
    #[error("Parsing error: {0}")]
    ParsingError(String),

    #[error(transparent)]
    ParsingJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    Error(#[from] anyhow::Error),

    #[error(transparent)]
    DataContractError(DataContractError),

    #[error(transparent)]
    AbstractConsensusError(Box<mocks::ConsensusError>),

    #[error(transparent)]
    Document(Box<DocumentError>),

    #[error("Generic Error: {0}")]
    Generic(String),

    #[error("Invalid Data Contract: {errors:?}")]
    InvalidDataContractError {
        errors: Vec<mocks::ConsensusError>,
        raw_data_contract: serde_json::Value,
    },
}

impl From<&str> for ProtocolError {
    fn from(v: &str) -> ProtocolError {
        ProtocolError::Generic(String::from(v))
    }
}

impl From<String> for ProtocolError {
    fn from(v: String) -> ProtocolError {
        Self::from(v.as_str())
    }
}

impl From<mocks::ConsensusError> for ProtocolError {
    fn from(e: mocks::ConsensusError) -> Self {
        ProtocolError::AbstractConsensusError(Box::new(e))
    }
}

impl From<DataContractError> for ProtocolError {
    fn from(e: DataContractError) -> Self {
        ProtocolError::DataContractError(e)
    }
}
