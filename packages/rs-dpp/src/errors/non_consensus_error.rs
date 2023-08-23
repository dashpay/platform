use platform_value::Error as ValueError;
use thiserror::Error;

use crate::version::FeatureVersion;
use crate::{
    CompatibleProtocolVersionIsNotDefinedError, DPPError, InvalidVectorSizeError, SerdeParsingError,
};

#[derive(Debug, Error)]
pub enum NonConsensusError {
    /// Value error
    #[error("value error: {0}")]
    ValueError(#[from] ValueError),
    /// Platform expected some specific versions
    #[error("non consensus unknown version on {method}, received: {received}")]
    UnknownVersionMismatch {
        /// method
        method: String,
        /// the allowed versions for this method
        known_versions: Vec<FeatureVersion>,
        /// requested core height
        received: FeatureVersion,
    },
    #[error("Unexpected serde parsing error: {0:#}")]
    SerdeParsingError(SerdeParsingError),
    #[error(transparent)]
    CompatibleProtocolVersionIsNotDefinedError(CompatibleProtocolVersionIsNotDefinedError),
    #[error("SerdeJsonError: {0}")]
    SerdeJsonError(String),
    #[error(transparent)]
    InvalidVectorSizeError(InvalidVectorSizeError),
    #[error("StateRepositoryFetchError: {0}")]
    StateRepositoryFetchError(String),
    #[error("WithdrawalError: {0}")]
    WithdrawalError(String),
    #[error("IdentifierCreateError: {0}")]
    IdentifierCreateError(String),
    #[error("StateTransitionCreationError: {0}")]
    StateTransitionCreationError(String),
    #[error("IdentityPublicKeyCreateError: {0}")]
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

    #[error(transparent)]
    DPPError(#[from] DPPError),

    #[error(transparent)]
    Error(#[from] anyhow::Error),

    /// Error
    #[error("overflow error: {0}")]
    Overflow(&'static str),
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
