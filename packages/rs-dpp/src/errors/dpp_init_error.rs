use std::borrow::Cow;

use jsonschema::ValidationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DashPlatformProtocolInitError {
    #[error("{0}")]
    SchemaDeserializationError(serde_json::Error),
    #[error("{0}")]
    ValidationError(ValidationError<'static>),
    #[error("Loaded Schema is invalid: {0}")]
    InvalidSchemaError(&'static str),
}

fn into_owned(err: ValidationError) -> ValidationError<'static> {
    ValidationError {
        instance_path: err.instance_path.clone(),
        instance: Cow::Owned(err.instance.into_owned()),
        kind: err.kind,
        schema_path: err.schema_path,
    }
}

impl From<serde_json::Error> for DashPlatformProtocolInitError {
    fn from(error: serde_json::Error) -> Self {
        Self::SchemaDeserializationError(error)
    }
}

impl<'a> From<ValidationError<'a>> for DashPlatformProtocolInitError {
    fn from(err: ValidationError<'a>) -> Self {
        Self::ValidationError(into_owned(err))
    }
}
