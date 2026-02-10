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
#[error("Anchor not found in the recorded anchors tree: {}", hex::encode(anchor))]
pub struct InvalidAnchorError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    anchor: [u8; 32],
}

impl InvalidAnchorError {
    pub fn new(anchor: [u8; 32]) -> Self {
        Self { anchor }
    }

    pub fn anchor(&self) -> [u8; 32] {
        self.anchor
    }
}

impl From<InvalidAnchorError> for ConsensusError {
    fn from(err: InvalidAnchorError) -> Self {
        Self::StateError(StateError::InvalidAnchorError(err))
    }
}
