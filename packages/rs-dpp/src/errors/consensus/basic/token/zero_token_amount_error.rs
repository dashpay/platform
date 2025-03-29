use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Token amount can't be 0")]
#[platform_serialize(unversioned)]
pub struct ZeroTokenAmountError {}

impl Default for ZeroTokenAmountError {
    fn default() -> Self {
        Self::new()
    }
}

impl ZeroTokenAmountError {
    /// Creates a new `ZeroTokenAmountError`.
    pub fn new() -> Self {
        Self {}
    }
}

impl From<ZeroTokenAmountError> for ConsensusError {
    fn from(err: ZeroTokenAmountError) -> Self {
        Self::BasicError(BasicError::ZeroTokenAmountError(err))
    }
}
