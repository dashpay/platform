use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("State transition size {actual_size_kbytes} KB is more than maximum {max_size_kbytes} KB")]
#[platform_serialize(unversioned)]
pub struct StateTransitionMaxSizeExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    actual_size_kbytes: usize,
    max_size_kbytes: usize,
}

impl StateTransitionMaxSizeExceededError {
    pub fn new(actual_size_kbytes: usize, max_size_kbytes: usize) -> Self {
        Self {
            actual_size_kbytes,
            max_size_kbytes,
        }
    }

    pub fn actual_size_kbytes(&self) -> usize {
        self.actual_size_kbytes
    }
    pub fn max_size_kbytes(&self) -> usize {
        self.max_size_kbytes
    }
}

impl From<StateTransitionMaxSizeExceededError> for ConsensusError {
    fn from(err: StateTransitionMaxSizeExceededError) -> Self {
        Self::BasicError(BasicError::StateTransitionMaxSizeExceededError(err))
    }
}
