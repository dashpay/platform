use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A moderator's field change that names no field, or names a system property.
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
    "The fields a moderator's document change sets are invalid: {}",
    message
)]
#[platform_serialize(unversioned)]
pub struct InvalidContractModerationDocumentFieldsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    message: String,
}

impl InvalidContractModerationDocumentFieldsError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<InvalidContractModerationDocumentFieldsError> for ConsensusError {
    fn from(err: InvalidContractModerationDocumentFieldsError) -> Self {
        Self::BasicError(BasicError::InvalidContractModerationDocumentFieldsError(
            err,
        ))
    }
}
