use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("State transition size {actual_size_kbytes} KB is more than maximum {max_size_kbytes} KB")]
pub struct StateTransitionMaxSizeExceededError {
    actual_size_kbytes: usize,
    max_size_kbytes: usize,
}

impl StateTransitionMaxSizeExceededError {
    pub fn new(actual_size_kbytes: usize, max_size_kbytes: usize) -> Self {
        Self {
            actual_size_kbytes,
            max_size_kbytes,
        }
    }

    pub fn actual_size_kbytes(&self) -> usize {
        self.actual_size_kbytes
    }
    pub fn max_size_kbytes(&self) -> usize {
        self.max_size_kbytes
    }
}

impl From<StateTransitionMaxSizeExceededError> for ConsensusError {
    fn from(err: StateTransitionMaxSizeExceededError) -> Self {
        Self::StateTransitionMaxSizeExceededError(err)
    }
}
