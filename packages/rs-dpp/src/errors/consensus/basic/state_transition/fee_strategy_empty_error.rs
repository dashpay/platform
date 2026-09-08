use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    DecodeUntrusted,
)]
#[error("Fee strategy must have at least one step")]
#[platform_serialize(unversioned)]
pub struct FeeStrategyEmptyError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
}

impl FeeStrategyEmptyError {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for FeeStrategyEmptyError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<FeeStrategyEmptyError> for ConsensusError {
    fn from(err: FeeStrategyEmptyError) -> Self {
        Self::BasicError(BasicError::FeeStrategyEmptyError(err))
    }
}
