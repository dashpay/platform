use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[error("State transition type is not present")]
pub struct MissingStateTransitionTypeError;

impl MissingStateTransitionTypeError {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<MissingStateTransitionTypeError> for ConsensusError {
    fn from(err: MissingStateTransitionTypeError) -> Self {
        Self::BasicError(BasicError::MissingStateTransitionTypeError(err))
    }
}
