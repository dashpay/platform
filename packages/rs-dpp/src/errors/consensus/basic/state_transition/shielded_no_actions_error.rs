use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Shielded transition must have at least one action")]
#[platform_serialize(unversioned)]
pub struct ShieldedNoActionsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
}

impl ShieldedNoActionsError {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ShieldedNoActionsError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<ShieldedNoActionsError> for ConsensusError {
    fn from(err: ShieldedNoActionsError) -> Self {
        Self::BasicError(BasicError::ShieldedNoActionsError(err))
    }
}
