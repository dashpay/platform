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
    "Document type {} charges an action fee for the moderators but the contract declares no moderation",
    document_type_name
)]
#[platform_serialize(unversioned)]
pub struct DocumentActionFeesWithoutModerationError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
}

impl DocumentActionFeesWithoutModerationError {
    pub fn new(document_type_name: String) -> Self {
        Self { document_type_name }
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }
}

impl From<DocumentActionFeesWithoutModerationError> for ConsensusError {
    fn from(err: DocumentActionFeesWithoutModerationError) -> Self {
        Self::BasicError(BasicError::DocumentActionFeesWithoutModerationError(err))
    }
}
