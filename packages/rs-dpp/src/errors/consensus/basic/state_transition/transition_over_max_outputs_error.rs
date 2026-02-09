use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("State transition has {actual_outputs} outputs, which exceeds the maximum allowed {max_outputs}")]
#[platform_serialize(unversioned)]
pub struct TransitionOverMaxOutputsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    actual_outputs: u16,
    max_outputs: u16,
}

impl TransitionOverMaxOutputsError {
    pub fn new(actual_outputs: u16, max_outputs: u16) -> Self {
        Self {
            actual_outputs,
            max_outputs,
        }
    }

    pub fn actual_outputs(&self) -> u16 {
        self.actual_outputs
    }

    pub fn max_outputs(&self) -> u16 {
        self.max_outputs
    }
}

impl From<TransitionOverMaxOutputsError> for ConsensusError {
    fn from(err: TransitionOverMaxOutputsError) -> Self {
        Self::BasicError(BasicError::TransitionOverMaxOutputsError(err))
    }
}
