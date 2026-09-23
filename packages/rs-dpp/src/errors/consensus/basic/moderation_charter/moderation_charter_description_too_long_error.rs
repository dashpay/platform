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
#[error("The moderation charter's description is {length} bytes long, the maximum is {max_length}")]
#[platform_serialize(unversioned)]
pub struct ModerationCharterDescriptionTooLongError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    length: u64,
    max_length: u16,
}

impl ModerationCharterDescriptionTooLongError {
    pub fn new(length: u64, max_length: u16) -> Self {
        Self { length, max_length }
    }

    /// The length of the description, in bytes of UTF-8
    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn max_length(&self) -> u16 {
        self.max_length
    }
}

impl From<ModerationCharterDescriptionTooLongError> for ConsensusError {
    fn from(err: ModerationCharterDescriptionTooLongError) -> Self {
        Self::BasicError(BasicError::ModerationCharterDescriptionTooLongError(err))
    }
}
