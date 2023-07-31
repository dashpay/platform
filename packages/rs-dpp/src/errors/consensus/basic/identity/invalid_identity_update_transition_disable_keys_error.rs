use crate::consensus::basic::BasicError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Identity Update Transition must specify timestamp when disabling keys")]
pub struct InvalidIdentityUpdateTransitionDisableKeysError;

impl InvalidIdentityUpdateTransitionDisableKeysError {
    pub fn new() -> Self {
        Self { }
    }
}

impl From<InvalidIdentityUpdateTransitionDisableKeysError> for ConsensusError {
    fn from(err: InvalidIdentityUpdateTransitionDisableKeysError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityUpdateTransitionDisableKeysError(err))
    }
}
