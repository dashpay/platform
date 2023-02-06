use thiserror::Error;

use crate::ProtocolError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
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

impl From<InvalidStateTransitionTypeError> for ProtocolError {
    fn from(err: InvalidStateTransitionTypeError) -> Self {
        Self::InvalidStateTransitionTypeError(err)
    }
}
