use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Inputs must be less than outputs for identity funding: input_sum={input_sum} but should be at least minimum_difference={minimum_difference} less than output_sum={output_sum}")]
#[platform_serialize(unversioned)]
pub struct InputsNotLessThanOutputsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    input_sum: u64,
    output_sum: u64,
    minimum_difference: u64,
}

impl InputsNotLessThanOutputsError {
    pub fn new(input_sum: u64, output_sum: u64, minimum_difference: u64) -> Self {
        Self {
            input_sum,
            output_sum,
            minimum_difference,
        }
    }

    pub fn input_sum(&self) -> u64 {
        self.input_sum
    }

    pub fn output_sum(&self) -> u64 {
        self.output_sum
    }

    pub fn minimum_difference(&self) -> u64 {
        self.minimum_difference
    }
}

impl From<InputsNotLessThanOutputsError> for ConsensusError {
    fn from(err: InputsNotLessThanOutputsError) -> Self {
        Self::BasicError(BasicError::InputsNotLessThanOutputsError(err))
    }
}
