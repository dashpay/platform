use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("document type name '{name}' is not supported. It must be from 1 to 64 alphanumeric chars, and '_' or '-'.")]
#[platform_serialize(unversioned)]
pub struct InvalidDocumentTypeNameError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    name: String,
}

impl InvalidDocumentTypeNameError {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl From<InvalidDocumentTypeNameError> for ConsensusError {
    fn from(err: InvalidDocumentTypeNameError) -> Self {
        Self::BasicError(BasicError::InvalidDocumentTypeNameError(err))
    }
}
