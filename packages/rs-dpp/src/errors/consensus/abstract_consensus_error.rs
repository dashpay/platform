use jsonschema::ValidationError;
use thiserror::Error;
use crate::errors::consensus::basic::JsonSchemaError;

pub trait AbstractConsensusError {

}

#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("`{0}`")]
    JsonSchemaError(JsonSchemaError)
}

impl<'a> From<ValidationError<'a>> for ConsensusError {
    fn from(validation_error: ValidationError<'a>) -> Self {
        Self::JsonSchemaError(JsonSchemaError::from(validation_error))
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