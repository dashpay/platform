use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Encode, Decode)]
#[error("State transition type is not present")]
pub struct MissingStateTransitionTypeError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MissingStateTransitionTypeError {
    pub fn new() -> Self {
        Self
    }
}

impl From<MissingStateTransitionTypeError> for ConsensusError {
    fn from(err: MissingStateTransitionTypeError) -> Self {
        Self::BasicError(BasicError::MissingStateTransitionTypeError(err))
    }
}
