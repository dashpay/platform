use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

// @append_only
#[derive(
    Error, Debug, PartialEq, Eq, PlatformSerialize, PlatformDeserialize, Encode, Decode, Clone,
)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub enum JsonSchemaError {
    #[error("can't create json schema: {0}")]
    CreateSchemaError(String),

    #[error("schema compatibility validation failed: {0}")]
    SchemaCompatibilityValidationError(String),
}
