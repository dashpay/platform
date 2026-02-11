use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
#[error(
    "shielded pool has insufficient notes for outgoing transition: pool has {current_count} notes but minimum {minimum_required} required"
)]
pub struct InsufficientPoolNotesError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    current_count: u64,
    minimum_required: u64,
}

impl InsufficientPoolNotesError {
    pub fn new(current_count: u64, minimum_required: u64) -> Self {
        Self {
            current_count,
            minimum_required,
        }
    }

    pub fn current_count(&self) -> u64 {
        self.current_count
    }

    pub fn minimum_required(&self) -> u64 {
        self.minimum_required
    }
}

impl From<InsufficientPoolNotesError> for ConsensusError {
    fn from(err: InsufficientPoolNotesError) -> Self {
        Self::StateError(StateError::InsufficientPoolNotesError(err))
    }
}
