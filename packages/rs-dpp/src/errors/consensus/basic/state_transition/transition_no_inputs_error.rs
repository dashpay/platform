use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("State transition must have at least one input")]
#[platform_serialize(unversioned)]
pub struct TransitionNoInputsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
}

impl TransitionNoInputsError {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for TransitionNoInputsError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TransitionNoInputsError> for ConsensusError {
    fn from(err: TransitionNoInputsError) -> Self {
        Self::BasicError(BasicError::TransitionNoInputsError(err))
    }
}
