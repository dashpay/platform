use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
)]
#[error("State transition type is not present")]
#[platform_serialize(unversioned)]
pub struct MissingStateTransitionTypeError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MissingStateTransitionTypeError {
    pub fn new() -> Self {
        Self
    }
}

impl From<MissingStateTransitionTypeError> for ConsensusError {
    fn from(err: MissingStateTransitionTypeError) -> Self {
        Self::BasicError(BasicError::MissingStateTransitionTypeError(err))
    }
}
