use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Token price can't be 0")]
#[platform_serialize(unversioned)]
pub struct ZeroTokenPriceError {}

impl Default for ZeroTokenPriceError {
    fn default() -> Self {
        Self::new()
    }
}

impl ZeroTokenPriceError {
    /// Creates a new `ZeroTokenPriceError`.
    pub fn new() -> Self {
        Self {}
    }
}

impl From<ZeroTokenPriceError> for ConsensusError {
    fn from(err: ZeroTokenPriceError) -> Self {
        Self::BasicError(BasicError::ZeroTokenPriceError(err))
    }
}
