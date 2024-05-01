use crate::version::FeatureVersion;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DashPlatformProtocolInitError {
    #[error(transparent)]
    SchemaDeserializationError(serde_json::Error),
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

impl From<serde_json::Error> for DashPlatformProtocolInitError {
    fn from(error: serde_json::Error) -> Self {
        Self::SchemaDeserializationError(error)
    }
}
