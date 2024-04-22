use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};

use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Amount of document transitions must be less or equal to {max_transitions}")]
#[platform_serialize(unversioned)]
pub struct MaxDocumentsTransitionsExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    max_transitions: u16,
}

impl MaxDocumentsTransitionsExceededError {
    pub fn new(max_transitions: u16) -> Self {
        Self { max_transitions }
    }

    pub fn max_transitions(&self) -> u16 {
        self.max_transitions
    }
}

impl From<MaxDocumentsTransitionsExceededError> for ConsensusError {
    fn from(err: MaxDocumentsTransitionsExceededError) -> Self {
        Self::BasicError(BasicError::MaxDocumentsTransitionsExceededError(err))
    }
}
