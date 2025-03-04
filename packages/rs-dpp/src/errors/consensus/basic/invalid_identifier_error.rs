use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid {identifier_name}: {message}")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InvalidIdentifierError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub identifier_name: String,
    pub message: String,
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
