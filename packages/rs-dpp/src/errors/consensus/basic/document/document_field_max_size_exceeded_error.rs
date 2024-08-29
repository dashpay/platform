use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Document field {field} size {actual_size_bytes} is more than system maximum {max_size_bytes}"
)]
#[platform_serialize(unversioned)]
pub struct DocumentFieldMaxSizeExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    field: String,
    actual_size_bytes: u64,
    max_size_bytes: u64,
}

impl DocumentFieldMaxSizeExceededError {
    pub fn new(field: String, actual_size_bytes: u64, max_size_bytes: u64) -> Self {
        Self {
            field,
            actual_size_bytes,
            max_size_bytes,
        }
    }

    pub fn field(&self) -> &str {
        self.field.as_str()
    }

    pub fn actual_size_bytes(&self) -> u64 {
        self.actual_size_bytes
    }
    pub fn max_size_bytes(&self) -> u64 {
        self.max_size_bytes
    }
}

impl From<DocumentFieldMaxSizeExceededError> for ConsensusError {
    fn from(err: DocumentFieldMaxSizeExceededError) -> Self {
        Self::BasicError(BasicError::DocumentFieldMaxSizeExceededError(err))
    }
}
