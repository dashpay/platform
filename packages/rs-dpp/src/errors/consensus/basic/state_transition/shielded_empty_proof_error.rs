use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Shielded transition proof must not be empty")]
#[platform_serialize(unversioned)]
pub struct ShieldedEmptyProofError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
}

impl ShieldedEmptyProofError {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ShieldedEmptyProofError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<ShieldedEmptyProofError> for ConsensusError {
    fn from(err: ShieldedEmptyProofError) -> Self {
        Self::BasicError(BasicError::ShieldedEmptyProofError(err))
    }
}
