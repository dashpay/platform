use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid {identifier_name}: {message}")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentifierError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identifier_name: String,
    message: String,
}

impl InvalidIdentifierError {
    pub fn new(identifier_name: String, message: String) -> Self {
        Self {
            identifier_name,
            message,
        }
    }

    pub fn identifier_name(&self) -> &str {
        &self.identifier_name
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<InvalidIdentifierError> for ConsensusError {
    fn from(err: InvalidIdentifierError) -> Self {
        Self::BasicError(BasicError::InvalidIdentifierError(err))
    }
}
