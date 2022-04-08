use std::borrow::Cow;
use std::path::{Display, Path};
use jsonschema::error::ValidationErrorKind;
use jsonschema::paths::{JSONPointer, PathChunk};
use jsonschema::ValidationError;
use thiserror::Error;
use crate::errors::consensus::{AbstractConsensusError, ConsensusError};

#[derive(Error, Debug)]
#[error("JsonSchemaError: {message:?}, kind: {kind:?}, instance_path: {instance_path:?}, schema_path:{schema_path:?}")]
pub struct JsonSchemaError {
    message: String,
    // TODO: deconstruct this - keyword is no a kind, but kind contains some additional data
    kind: ValidationErrorKind,
    instance_path: JSONPointer,
    schema_path: JSONPointer,
    // This is stored inside the keyword (error kind)
    // this.params = params;
    // this.propertyName = propertyName;
}

impl AbstractConsensusError for JsonSchemaError {}

impl<'a> From<ValidationError<'a>> for JsonSchemaError {
    fn from(validation_error: ValidationError<'a>) -> Self {
        Self {
            // TODO: implement message
            message: String::new(),
            kind: validation_error.kind,
            instance_path: validation_error.instance_path,
            schema_path: validation_error.schema_path
        }
    }
}

impl JsonSchemaError {
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn instance_path(&self) -> &JSONPointer {
        &self.instance_path
    }

    pub fn schema_path(&self) -> &JSONPointer {
        &self.schema_path
    }

    pub fn kind(&self) -> &ValidationErrorKind {
        &self.kind
    }

    pub fn keyword(&self) -> Option<&str> {
        let chunk = self.schema_path.last()?;
        match chunk {
            PathChunk::Property(_) => { None }
            PathChunk::Index(_) => { None }
            PathChunk::Keyword(keyword) => {Some(keyword)}
        }
    }

    // Kind was called "params" in the original reference
    pub fn params(&self) -> &ValidationErrorKind {
        self.kind()
    }
}