use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Amount of document transitions must be less or equal to {max_transitions}")]
pub struct MaxDocumentsTransitionsExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    max_transitions: u32,
}

impl MaxDocumentsTransitionsExceededError {
    pub fn new(max_transitions: u32) -> Self {
        Self { max_transitions }
    }

    pub fn max_transitions(&self) -> u32 {
        self.max_transitions
    }
}

impl From<MaxDocumentsTransitionsExceededError> for ConsensusError {
    fn from(err: MaxDocumentsTransitionsExceededError) -> Self {
        Self::BasicError(BasicError::MaxDocumentsTransitionsExceededError(err))
    }
}
