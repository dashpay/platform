use crate::consensus::basic::BasicError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Identity Update Transition neither contains new public keys or key ids to disable")]
pub struct InvalidIdentityUpdateTransitionEmptyError;

impl InvalidIdentityUpdateTransitionEmptyError {
    pub fn new() -> Self {
        Self { }
    }
}

impl From<InvalidIdentityUpdateTransitionEmptyError> for ConsensusError {
    fn from(err: InvalidIdentityUpdateTransitionEmptyError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityUpdateTransitionEmptyError(err))
    }
}
