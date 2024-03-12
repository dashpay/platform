use bincode::{Decode, Encode};
use thiserror::Error;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use crate::ProtocolError;

// @append_only
#[derive(Error, Debug, PlatformSerialize, PlatformDeserialize, Encode, Decode, Clone)]
pub enum JsonSchemaError {
    #[error("can't create json schema: {0}")]
    CreateSchemaError(String),
}
