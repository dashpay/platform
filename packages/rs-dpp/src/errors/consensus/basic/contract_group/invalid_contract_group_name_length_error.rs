use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
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
#[error("Contract group name has invalid length {}: it must be between 1 and {} characters", name.chars().count(), max_length)]
#[platform_serialize(unversioned)]
pub struct InvalidContractGroupNameLengthError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    name: String,
    max_length: u16,
}

impl InvalidContractGroupNameLengthError {
    pub fn new(name: String, max_length: u16) -> Self {
        Self { name, max_length }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn max_length(&self) -> u16 {
        self.max_length
    }
}

impl From<InvalidContractGroupNameLengthError> for ConsensusError {
    fn from(err: InvalidContractGroupNameLengthError) -> Self {
        Self::BasicError(BasicError::InvalidContractGroupNameLengthError(err))
    }
}
