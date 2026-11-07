use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// Two entries of a `PrivateDocumentCreateFromShieldedPool` are the same bytes (protocol version
/// 15). A recipient keeps one copy of an entry, so a second copy in the same transition would
/// only be stored for nothing, and Drive appends every entry of a transition in one batch, behind
/// the same header, where two identical appends are the same operation twice.
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
    "Entry {entry_index} of a private document create is the same as entry {first_index}; \
     every entry of a transition must differ"
)]
#[platform_serialize(unversioned)]
pub struct DuplicatePrivateDocumentEntryError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    first_index: u16,
    entry_index: u16,
}

impl DuplicatePrivateDocumentEntryError {
    pub fn new(first_index: u16, entry_index: u16) -> Self {
        Self {
            first_index,
            entry_index,
        }
    }

    pub fn first_index(&self) -> u16 {
        self.first_index
    }

    pub fn entry_index(&self) -> u16 {
        self.entry_index
    }
}

impl From<DuplicatePrivateDocumentEntryError> for ConsensusError {
    fn from(err: DuplicatePrivateDocumentEntryError) -> Self {
        Self::BasicError(BasicError::DuplicatePrivateDocumentEntryError(err))
    }
}
