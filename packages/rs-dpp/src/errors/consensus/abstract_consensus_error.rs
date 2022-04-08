use jsonschema::ValidationError;
use thiserror::Error;
use crate::errors::consensus::basic::{IncompatibleProtocolVersionError, JsonSchemaError, UnsupportedProtocolVersionError};

// TODO: move this to a separate get codes function, as it's done in js dpp
fn get_error_code(error: ConsensusError) -> u32 {
    match error {
        ConsensusError::JsonSchemaError(_) => { 1005 }
        ConsensusError::UnsupportedProtocolVersionError(_) => { 1002 }
        ConsensusError::IncompatibleProtocolVersionError(_) => { 1003 }
    }
}

pub trait AbstractConsensusError: Into<ConsensusError> {
    fn get_code(&self, error: Self) -> u32 {
        get_error_code(error.into())
    }
}

#[derive(Error, Debug)]
#[error("`{0}`")]
pub enum ConsensusError {
    #[error("`{0}`")]
    JsonSchemaError(JsonSchemaError),
    #[error("`{0}`")]
    UnsupportedProtocolVersionError(UnsupportedProtocolVersionError),
    #[error("`{0}`")]
    IncompatibleProtocolVersionError(IncompatibleProtocolVersionError),
}

impl<'a> From<ValidationError<'a>> for ConsensusError {
    fn from(validation_error: ValidationError<'a>) -> Self {
        Self::JsonSchemaError(JsonSchemaError::from(validation_error))
    }
}

impl From<JsonSchemaError> for ConsensusError {
    fn from(json_schema_error: JsonSchemaError) -> Self {
        Self::JsonSchemaError(json_schema_error)
    }
}

impl From<UnsupportedProtocolVersionError> for ConsensusError {
    fn from(error: UnsupportedProtocolVersionError) -> Self {
        Self::UnsupportedProtocolVersionError(error)
    }
}

impl From<IncompatibleProtocolVersionError> for ConsensusError {
    fn from(error: IncompatibleProtocolVersionError) -> Self {
        Self::IncompatibleProtocolVersionError(error)
    }
}

impl ConsensusError {
    pub fn json_schema_error(&self) -> Option<&JsonSchemaError> {
        match self {
            ConsensusError::JsonSchemaError(err) => { Some(err) }
            _ => None
        }
    }
}