use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Main group is not defined.")]
#[platform_serialize(unversioned)]
pub struct MainGroupIsNotDefinedError;

impl Default for MainGroupIsNotDefinedError {
    fn default() -> Self {
        Self::new()
    }
}

impl MainGroupIsNotDefinedError {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<MainGroupIsNotDefinedError> for ConsensusError {
    fn from(err: MainGroupIsNotDefinedError) -> Self {
        Self::BasicError(BasicError::MainGroupIsNotDefinedError(err))
    }
}
