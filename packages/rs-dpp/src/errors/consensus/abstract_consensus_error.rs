use jsonschema::ValidationError;
use thiserror::Error;
use crate::errors::consensus::basic::JsonSchemaError;

pub trait AbstractConsensusError {

}

#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("Please implement me")]
    JsonSchemaError(JsonSchemaError)
}

impl<'a> From<ValidationError<'a>> for ConsensusError {
    fn from(validation_error: ValidationError<'a>) -> Self {
        Self::JsonSchemaError(JsonSchemaError::from(validation_error))
    }
}