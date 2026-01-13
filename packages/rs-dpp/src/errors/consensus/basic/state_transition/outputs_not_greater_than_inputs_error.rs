use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Outputs must be greater than inputs for asset lock funding: inputs={input_sum}, outputs={output_sum}")]
#[platform_serialize(unversioned)]
pub struct OutputsNotGreaterThanInputsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    input_sum: u64,
    output_sum: u64,
}

impl OutputsNotGreaterThanInputsError {
    pub fn new(input_sum: u64, output_sum: u64) -> Self {
        Self {
            input_sum,
            output_sum,
        }
    }

    pub fn input_sum(&self) -> u64 {
        self.input_sum
    }

    pub fn output_sum(&self) -> u64 {
        self.output_sum
    }
}

impl From<OutputsNotGreaterThanInputsError> for ConsensusError {
    fn from(err: OutputsNotGreaterThanInputsError) -> Self {
        Self::BasicError(BasicError::OutputsNotGreaterThanInputsError(err))
    }
}
