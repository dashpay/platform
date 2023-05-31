use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Invalid State Transition type {transition_type}")]
pub struct InvalidStateTransitionTypeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    transition_type: u8,
}

impl InvalidStateTransitionTypeError {
    pub fn new(transition_type: u8) -> Self {
        Self { transition_type }
    }

    pub fn transition_type(&self) -> u8 {
        self.transition_type
    }
}

impl From<InvalidStateTransitionTypeError> for ConsensusError {
    fn from(err: InvalidStateTransitionTypeError) -> Self {
        Self::BasicError(BasicError::InvalidStateTransitionTypeError(err))
    }
}
