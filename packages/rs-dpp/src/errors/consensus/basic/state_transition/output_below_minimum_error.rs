use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Output amount {output_amount} is below minimum {minimum_amount}")]
#[platform_serialize(unversioned)]
pub struct OutputBelowMinimumError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    output_amount: u64,
    minimum_amount: u64,
}

impl OutputBelowMinimumError {
    pub fn new(output_amount: u64, minimum_amount: u64) -> Self {
        Self {
            output_amount,
            minimum_amount,
        }
    }

    pub fn output_amount(&self) -> u64 {
        self.output_amount
    }

    pub fn minimum_amount(&self) -> u64 {
        self.minimum_amount
    }
}

impl From<OutputBelowMinimumError> for ConsensusError {
    fn from(err: OutputBelowMinimumError) -> Self {
        Self::BasicError(BasicError::OutputBelowMinimumError(err))
    }
}
