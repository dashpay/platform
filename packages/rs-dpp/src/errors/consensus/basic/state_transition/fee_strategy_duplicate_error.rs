use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Fee strategy contains duplicate entries")]
#[platform_serialize(unversioned)]
pub struct FeeStrategyDuplicateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
}

impl FeeStrategyDuplicateError {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for FeeStrategyDuplicateError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<FeeStrategyDuplicateError> for ConsensusError {
    fn from(err: FeeStrategyDuplicateError) -> Self {
        Self::BasicError(BasicError::FeeStrategyDuplicateError(err))
    }
}
