use crate::consensus::basic::BasicError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Invalid State Transition type {transition_type}")]
pub struct InvalidStateTransitionTypeError {
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
