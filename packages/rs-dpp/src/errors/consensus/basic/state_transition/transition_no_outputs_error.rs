use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("State transition must have at least one output")]
#[platform_serialize(unversioned)]
pub struct TransitionNoOutputsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
}

impl TransitionNoOutputsError {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for TransitionNoOutputsError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TransitionNoOutputsError> for ConsensusError {
    fn from(err: TransitionNoOutputsError) -> Self {
        Self::BasicError(BasicError::TransitionNoOutputsError(err))
    }
}
