use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Number of inputs ({input_count}) does not match number of witnesses ({witness_count})")]
#[platform_serialize(unversioned)]
pub struct InputWitnessCountMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    input_count: u16,
    witness_count: u16,
}

impl InputWitnessCountMismatchError {
    pub fn new(input_count: u16, witness_count: u16) -> Self {
        Self {
            input_count,
            witness_count,
        }
    }

    pub fn input_count(&self) -> u16 {
        self.input_count
    }

    pub fn witness_count(&self) -> u16 {
        self.witness_count
    }
}

impl From<InputWitnessCountMismatchError> for ConsensusError {
    fn from(err: InputWitnessCountMismatchError) -> Self {
        Self::BasicError(BasicError::InputWitnessCountMismatchError(err))
    }
}
