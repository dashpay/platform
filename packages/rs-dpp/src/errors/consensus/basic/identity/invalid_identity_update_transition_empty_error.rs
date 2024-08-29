use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity Update Transition neither contains new public keys or key ids to disable")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityUpdateTransitionEmptyError;

impl Default for InvalidIdentityUpdateTransitionEmptyError {
    fn default() -> Self {
        Self::new()
    }
}

impl InvalidIdentityUpdateTransitionEmptyError {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<InvalidIdentityUpdateTransitionEmptyError> for ConsensusError {
    fn from(err: InvalidIdentityUpdateTransitionEmptyError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityUpdateTransitionEmptyError(err))
    }
}
