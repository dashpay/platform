use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

// @append_only
#[derive(
    Error,
    Debug,
    PartialEq,
    Eq,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    Encode,
    Decode,
    Clone,
    DecodeUntrusted,
)]
pub enum JsonSchemaError {
    #[error("can't create json schema: {0}")]
    CreateSchemaError(String),

    #[error("schema compatibility validation failed: {0}")]
    SchemaCompatibilityValidationError(String),
}
