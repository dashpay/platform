use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
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
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
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
