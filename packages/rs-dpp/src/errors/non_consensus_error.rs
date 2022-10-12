use thiserror::Error;

use crate::{
    CompatibleProtocolVersionIsNotDefinedError, InvalidVectorSizeError, SerdeParsingError,
};

#[derive(Debug, Error, Clone)]
pub enum NonConsensusError {
    #[error("Unexpected serde parsing error: {0:#}")]
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
    #[error("{0}")]
    IdentityPublicKeyCreateError(String),

    /// When dynamic `Value` is validated it requires some specific properties to properly work
    #[error("The property is required: '{property_name}'")]
    RequiredPropertyError { property_name: String },

    /// Invalid or unsupported object has been used with function/method
    #[error("Invalid Data: {0}")]
    InvalidDataProcessedError(String),

    #[error("Failed to create a new instance of '{object_name}'': {details}")]
    ObjectCreationError {
        object_name: &'static str,
        details: String,
    },
}

pub mod object_names {
    pub const STATE_TRANSITION: &str = "State Transition";
}

impl NonConsensusError {
    pub fn object_creation_error(object_name: &'static str, error: impl std::fmt::Display) -> Self {
        Self::ObjectCreationError {
            object_name,
            details: format!("{}", error),
        }
    }
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
