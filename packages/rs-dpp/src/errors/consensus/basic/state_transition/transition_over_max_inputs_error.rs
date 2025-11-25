use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("State transition has {actual_inputs} inputs, which exceeds the maximum allowed {max_inputs}")]
#[platform_serialize(unversioned)]
pub struct TransitionOverMaxInputsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    actual_inputs: u16,
    max_inputs: u16,
}

impl TransitionOverMaxInputsError {
    pub fn new(actual_inputs: u16, max_inputs: u16) -> Self {
        Self {
            actual_inputs,
            max_inputs,
        }
    }

    pub fn actual_inputs(&self) -> u16 {
        self.actual_inputs
    }

    pub fn max_inputs(&self) -> u16 {
        self.max_inputs
    }
}

impl From<TransitionOverMaxInputsError> for ConsensusError {
    fn from(err: TransitionOverMaxInputsError) -> Self {
        Self::BasicError(BasicError::TransitionOverMaxInputsError(err))
    }
}
