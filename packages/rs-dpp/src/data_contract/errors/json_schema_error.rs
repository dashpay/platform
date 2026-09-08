use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

// @append_only
#[derive(
    Error, Debug, PartialEq, Eq, PlatformSerialize, PlatformDeserialize, Encode, Decode, Clone,
)]
// Derived separately so the append-only derive list above stays unchanged.
#[derive(DecodeUntrusted)]
pub enum JsonSchemaError {
    #[error("can't create json schema: {0}")]
    CreateSchemaError(String),

    #[error("schema compatibility validation failed: {0}")]
    SchemaCompatibilityValidationError(String),
}
