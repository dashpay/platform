use std::borrow::Cow;

use crate::version::FeatureVersion;
use jsonschema::ValidationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DashPlatformProtocolInitError {
    #[error(transparent)]
    SchemaDeserializationError(serde_json::Error),
    #[error(transparent)]
    ValidationError(ValidationError<'static>),
    #[error("Loaded Schema is invalid: {0}")]
    InvalidSchemaError(&'static str),
    #[error("platform init unknown version on {method}, received: {received}")]
    UnknownVersionMismatch {
        /// method
        method: String,
        /// the allowed versions for this method
        known_versions: Vec<FeatureVersion>,
        /// requested core height
        received: FeatureVersion,
    },
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
