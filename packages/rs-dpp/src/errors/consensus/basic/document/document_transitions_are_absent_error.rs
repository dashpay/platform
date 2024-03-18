use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};

use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Documents Batch Transition has no document transitions")]
#[platform_serialize(unversioned)]
pub struct DocumentTransitionsAreAbsentError {}

impl Default for DocumentTransitionsAreAbsentError {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentTransitionsAreAbsentError {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<DocumentTransitionsAreAbsentError> for ConsensusError {
    fn from(err: DocumentTransitionsAreAbsentError) -> Self {
        Self::BasicError(BasicError::DocumentTransitionsAreAbsentError(err))
    }
}
