use crate::errors::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity Update Transition must specify timestamp when disabling keys")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InvalidIdentityUpdateTransitionDisableKeysError;

impl Default for InvalidIdentityUpdateTransitionDisableKeysError {
    fn default() -> Self {
        Self::new()
    }
}

impl InvalidIdentityUpdateTransitionDisableKeysError {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<InvalidIdentityUpdateTransitionDisableKeysError> for ConsensusError {
    fn from(err: InvalidIdentityUpdateTransitionDisableKeysError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityUpdateTransitionDisableKeysError(
            err,
        ))
    }
}
