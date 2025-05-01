use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid token amount {}, should be between 1 and maximum allowed {}",
    token_amount,
    max_token_amount
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenAmountError {
    max_token_amount: u64,
    token_amount: u64,
}

impl InvalidTokenAmountError {
    /// Creates a new `InvalidTokenAmountError`.
    pub fn new(max_token_amount: u64, token_amount: u64) -> Self {
        Self {
            max_token_amount,
            token_amount,
        }
    }

    /// Returns the maximum allowed token amount.
    pub fn max_token_amount(&self) -> u64 {
        self.max_token_amount
    }

    /// Returns the invalid token amount that was provided.
    pub fn token_amount(&self) -> u64 {
        self.token_amount
    }
}

impl From<InvalidTokenAmountError> for ConsensusError {
    fn from(err: InvalidTokenAmountError) -> Self {
        Self::BasicError(BasicError::InvalidTokenAmountError(err))
    }
}
