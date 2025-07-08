use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "The specified new authorized action taker main group is not set in the token configuration"
)]
#[platform_serialize(unversioned)]
pub struct NewAuthorizedActionTakerMainGroupNotSetError {}

impl Default for NewAuthorizedActionTakerMainGroupNotSetError {
    fn default() -> Self {
        Self::new()
    }
}

impl NewAuthorizedActionTakerMainGroupNotSetError {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<NewAuthorizedActionTakerMainGroupNotSetError> for ConsensusError {
    fn from(err: NewAuthorizedActionTakerMainGroupNotSetError) -> Self {
        Self::StateError(StateError::NewAuthorizedActionTakerMainGroupNotSetError(
            err,
        ))
    }
}
