use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Unshield transition amount must be greater than zero")]
#[platform_serialize(unversioned)]
pub struct UnshieldAmountZeroError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
}

impl UnshieldAmountZeroError {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for UnshieldAmountZeroError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<UnshieldAmountZeroError> for ConsensusError {
    fn from(err: UnshieldAmountZeroError) -> Self {
        Self::BasicError(BasicError::UnshieldAmountZeroError(err))
    }
}
