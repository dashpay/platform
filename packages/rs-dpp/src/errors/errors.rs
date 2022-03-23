use crate::data_contract::errors::*;
use crate::document::errors::*;
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
    AbstractConsensusError(AbstractConsensusErrorMock),
    #[error(transparent)]
    Document(Box<DocumentError>),
}

impl From<AbstractConsensusErrorMock> for ProtocolError {
    fn from(e: AbstractConsensusErrorMock) -> Self {
        ProtocolError::AbstractConsensusError(e)
    }
}

impl From<DataContractError> for ProtocolError {
    fn from(e: DataContractError) -> Self {
        ProtocolError::DataContractError(e)
    }
}

// TODO implement
#[derive(Error, Debug)]
pub enum AbstractConsensusErrorMock {}
