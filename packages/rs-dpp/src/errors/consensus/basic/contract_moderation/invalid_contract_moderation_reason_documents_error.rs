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
    "The documents a contract moderation reason cites are invalid: {}",
    message
)]
#[platform_serialize(unversioned)]
pub struct InvalidContractModerationReasonDocumentsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    message: String,
}

impl InvalidContractModerationReasonDocumentsError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<InvalidContractModerationReasonDocumentsError> for ConsensusError {
    fn from(err: InvalidContractModerationReasonDocumentsError) -> Self {
        Self::BasicError(BasicError::InvalidContractModerationReasonDocumentsError(
            err,
        ))
    }
}
