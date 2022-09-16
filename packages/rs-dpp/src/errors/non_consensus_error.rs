use thiserror::Error;

use crate::{
    CompatibleProtocolVersionIsNotDefinedError, InvalidVectorSizeError, SerdeParsingError,
};

#[derive(Debug, Error, Clone)]
pub enum NonConsensusError {
    #[error("Unexpected serde parsing error")]
    SerdeParsingError(SerdeParsingError),
    #[error("{0}")]
    CompatibleProtocolVersionIsNotDefinedError(CompatibleProtocolVersionIsNotDefinedError),
    #[error("{0}")]
    SerdeJsonError(String),
    #[error("{0}")]
    InvalidVectorSizeError(InvalidVectorSizeError),
    #[error("{0}")]
    StateRepositoryFetchError(String),
    #[error("{0}")]
    IdentifierCreateError(String),

    #[error("The property is required: '{property_name}'")]
    RequiredPropertyError { property_name: String },
}

impl From<SerdeParsingError> for NonConsensusError {
    fn from(err: SerdeParsingError) -> Self {
        Self::SerdeParsingError(err)
    }
}

impl From<CompatibleProtocolVersionIsNotDefinedError> for NonConsensusError {
    fn from(err: CompatibleProtocolVersionIsNotDefinedError) -> Self {
        Self::CompatibleProtocolVersionIsNotDefinedError(err)
    }
}

impl From<serde_json::Error> for NonConsensusError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerdeJsonError(err.to_string())
    }
}

impl From<InvalidVectorSizeError> for NonConsensusError {
    fn from(err: InvalidVectorSizeError) -> Self {
        Self::InvalidVectorSizeError(err)
    }
}
