use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("State transition size {actual_size_bytes} is more than maximum {max_size_bytes}")]
#[platform_serialize(unversioned)]
pub struct StateTransitionMaxSizeExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    actual_size_bytes: u64,
    max_size_bytes: u64,
}

impl StateTransitionMaxSizeExceededError {
    pub fn new(actual_size_bytes: u64, max_size_bytes: u64) -> Self {
        Self {
            actual_size_bytes,
            max_size_bytes,
        }
    }

    pub fn actual_size_bytes(&self) -> u64 {
        self.actual_size_bytes
    }
    pub fn max_size_bytes(&self) -> u64 {
        self.max_size_bytes
    }
}

impl From<StateTransitionMaxSizeExceededError> for ConsensusError {
    fn from(err: StateTransitionMaxSizeExceededError) -> Self {
        Self::BasicError(BasicError::StateTransitionMaxSizeExceededError(err))
    }
}
