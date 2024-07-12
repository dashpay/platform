use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Parsing of serialized object failed due to: {parsing_error}")]
#[platform_serialize(unversioned)]
pub struct SerializedObjectParsingError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    parsing_error: String,
}

impl SerializedObjectParsingError {
    pub fn new(parsing_error: String) -> Self {
        Self { parsing_error }
    }

    pub fn parsing_error(&self) -> &str {
        &self.parsing_error
    }
}
impl From<SerializedObjectParsingError> for ConsensusError {
    fn from(err: SerializedObjectParsingError) -> Self {
        Self::BasicError(BasicError::SerializedObjectParsingError(err))
    }
}
