use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    DecodeUntrusted,
)]
#[error("JsonSchema compilation error: {compilation_error}")]
#[platform_serialize(unversioned)]
pub struct JsonSchemaCompilationError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    compilation_error: String,
}

impl JsonSchemaCompilationError {
    pub fn new(compilation_error: String) -> Self {
        Self { compilation_error }
    }

    pub fn compilation_error(&self) -> &str {
        &self.compilation_error
    }
}
impl From<JsonSchemaCompilationError> for ConsensusError {
    fn from(err: JsonSchemaCompilationError) -> Self {
        Self::BasicError(BasicError::JsonSchemaCompilationError(err))
    }
}
