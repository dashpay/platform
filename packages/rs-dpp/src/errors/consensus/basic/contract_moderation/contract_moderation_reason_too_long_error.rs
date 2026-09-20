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
#[error(
    "The text of a contract moderation reason is {} bytes long, the maximum is {}",
    length,
    max_length
)]
#[platform_serialize(unversioned)]
pub struct ContractModerationReasonTooLongError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    length: u64,
    max_length: u16,
}

impl ContractModerationReasonTooLongError {
    pub fn new(length: u64, max_length: u16) -> Self {
        Self { length, max_length }
    }

    /// The length of the text, in bytes of UTF-8
    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn max_length(&self) -> u16 {
        self.max_length
    }
}

impl From<ContractModerationReasonTooLongError> for ConsensusError {
    fn from(err: ContractModerationReasonTooLongError) -> Self {
        Self::BasicError(BasicError::ContractModerationReasonTooLongError(err))
    }
}
