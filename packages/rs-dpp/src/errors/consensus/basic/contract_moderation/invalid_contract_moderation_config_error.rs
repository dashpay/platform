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
#[error("Invalid contract moderation config: {}", reason)]
#[platform_serialize(unversioned)]
pub struct InvalidContractModerationConfigError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    reason: String,
}

impl InvalidContractModerationConfigError {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<InvalidContractModerationConfigError> for ConsensusError {
    fn from(err: InvalidContractModerationConfigError) -> Self {
        Self::BasicError(BasicError::InvalidContractModerationConfigError(err))
    }
}
