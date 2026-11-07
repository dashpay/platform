use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A `PrivateDocumentCreateFromShieldedPool` carries no entry, or more entries than
/// `SystemLimits::max_private_document_entries_per_transition` (protocol version 15).
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
#[error("A private document create carries {count} entries; it must carry from 1 to {max_count}")]
#[platform_serialize(unversioned)]
pub struct InvalidPrivateDocumentEntryCountError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    count: u64,
    max_count: u16,
}

impl InvalidPrivateDocumentEntryCountError {
    pub fn new(count: u64, max_count: u16) -> Self {
        Self { count, max_count }
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn max_count(&self) -> u16 {
        self.max_count
    }
}

impl From<InvalidPrivateDocumentEntryCountError> for ConsensusError {
    fn from(err: InvalidPrivateDocumentEntryCountError) -> Self {
        Self::BasicError(BasicError::InvalidPrivateDocumentEntryCountError(err))
    }
}
