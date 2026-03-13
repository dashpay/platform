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
#[error("Nullifier has already been spent: {}", hex::encode(nullifier))]
pub struct NullifierAlreadySpentError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    nullifier: [u8; 32],
}

impl NullifierAlreadySpentError {
    pub fn new(nullifier: [u8; 32]) -> Self {
        Self { nullifier }
    }

    pub fn nullifier(&self) -> [u8; 32] {
        self.nullifier
    }
}

impl From<NullifierAlreadySpentError> for ConsensusError {
    fn from(err: NullifierAlreadySpentError) -> Self {
        Self::StateError(StateError::NullifierAlreadySpentError(err))
    }
}
