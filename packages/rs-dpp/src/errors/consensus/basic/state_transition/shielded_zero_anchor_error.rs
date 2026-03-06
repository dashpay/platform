use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Shielded transition anchor must not be all zeros")]
#[platform_serialize(unversioned)]
pub struct ShieldedZeroAnchorError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
}

impl ShieldedZeroAnchorError {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ShieldedZeroAnchorError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<ShieldedZeroAnchorError> for ConsensusError {
    fn from(err: ShieldedZeroAnchorError) -> Self {
        Self::BasicError(BasicError::ShieldedZeroAnchorError(err))
    }
}
