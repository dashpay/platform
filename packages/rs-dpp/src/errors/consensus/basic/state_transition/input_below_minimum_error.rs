use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Input amount {input_amount} is below minimum {minimum_amount}")]
#[platform_serialize(unversioned)]
pub struct InputBelowMinimumError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    input_amount: u64,
    minimum_amount: u64,
}

impl InputBelowMinimumError {
    pub fn new(input_amount: u64, minimum_amount: u64) -> Self {
        Self {
            input_amount,
            minimum_amount,
        }
    }

    pub fn input_amount(&self) -> u64 {
        self.input_amount
    }

    pub fn minimum_amount(&self) -> u64 {
        self.minimum_amount
    }
}

impl From<InputBelowMinimumError> for ConsensusError {
    fn from(err: InputBelowMinimumError) -> Self {
        Self::BasicError(BasicError::InputBelowMinimumError(err))
    }
}
