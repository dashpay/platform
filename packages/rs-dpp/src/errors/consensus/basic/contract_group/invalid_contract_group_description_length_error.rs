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
#[error("Contract group description has invalid length {}: it must be between 1 and {} characters", description.chars().count(), max_length)]
#[platform_serialize(unversioned)]
pub struct InvalidContractGroupDescriptionLengthError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    description: String,
    max_length: u16,
}

impl InvalidContractGroupDescriptionLengthError {
    pub fn new(description: String, max_length: u16) -> Self {
        Self {
            description,
            max_length,
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn max_length(&self) -> u16 {
        self.max_length
    }
}

impl From<InvalidContractGroupDescriptionLengthError> for ConsensusError {
    fn from(err: InvalidContractGroupDescriptionLengthError) -> Self {
        Self::BasicError(BasicError::InvalidContractGroupDescriptionLengthError(err))
    }
}
