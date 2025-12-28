use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Exactly one output must have None value (remainder recipient), found {actual_count}")]
#[platform_serialize(unversioned)]
pub struct InvalidRemainderOutputCountError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    actual_count: u16,
}

impl InvalidRemainderOutputCountError {
    pub fn new(actual_count: u16) -> Self {
        Self { actual_count }
    }

    pub fn actual_count(&self) -> u16 {
        self.actual_count
    }
}

impl From<InvalidRemainderOutputCountError> for ConsensusError {
    fn from(err: InvalidRemainderOutputCountError) -> Self {
        Self::BasicError(BasicError::InvalidRemainderOutputCountError(err))
    }
}
