use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data contract is trying to define a token with {decimals} decimal places, which exceeds the maximum allowed: {max_decimals}.")]
#[platform_serialize(unversioned)]
pub struct DecimalsOverLimitError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    decimals: u8,
    max_decimals: u8,
}

impl DecimalsOverLimitError {
    pub fn new(decimals: u8, max_decimals: u8) -> Self {
        Self {
            decimals,
            max_decimals,
        }
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    pub fn max_decimals(&self) -> u8 {
        self.max_decimals
    }
}

impl From<DecimalsOverLimitError> for ConsensusError {
    fn from(err: DecimalsOverLimitError) -> Self {
        Self::BasicError(BasicError::DecimalsOverLimitError(err))
    }
}
