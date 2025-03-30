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
    "Invalid token price {}, exceeds maximum allowed {}",
    token_price,
    max_token_price
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenPriceError {
    max_token_price: u64,
    token_price: u64,
}

impl InvalidTokenPriceError {
    /// Creates a new `InvalidTokenPriceError`.
    pub fn new(max_token_price: u64, token_price: u64) -> Self {
        Self {
            max_token_price,
            token_price,
        }
    }

    /// Returns the maximum allowed token price.
    pub fn max_token_price(&self) -> u64 {
        self.max_token_price
    }

    /// Returns the invalid token price that was provided.
    pub fn token_price(&self) -> u64 {
        self.token_price
    }
}

impl From<InvalidTokenPriceError> for ConsensusError {
    fn from(err: InvalidTokenPriceError) -> Self {
        Self::BasicError(BasicError::InvalidTokenPriceError(err))
    }
}
