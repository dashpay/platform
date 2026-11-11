use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// An entry of a `PrivateDocumentCreateFromShieldedPool` is empty, takes more bytes than
/// `SystemLimits::max_private_document_entry_size`, or takes a different number of bytes than
/// the transition's first entry (protocol version 15). Every entry of a private document type
/// takes exactly the type's `entrySize`, which only the contract states; this is the check that
/// needs no contract.
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
    "Entry {entry_index} of a private document create takes {size} bytes; every entry must \
     take as many bytes as the first ({first_entry_size}), from 1 to {max_size}"
)]
#[platform_serialize(unversioned)]
pub struct InvalidPrivateDocumentEntrySizeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    entry_index: u16,
    size: u32,
    first_entry_size: u32,
    max_size: u32,
}

impl InvalidPrivateDocumentEntrySizeError {
    pub fn new(entry_index: u16, size: u32, first_entry_size: u32, max_size: u32) -> Self {
        Self {
            entry_index,
            size,
            first_entry_size,
            max_size,
        }
    }

    pub fn entry_index(&self) -> u16 {
        self.entry_index
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn first_entry_size(&self) -> u32 {
        self.first_entry_size
    }

    pub fn max_size(&self) -> u32 {
        self.max_size
    }
}

impl From<InvalidPrivateDocumentEntrySizeError> for ConsensusError {
    fn from(err: InvalidPrivateDocumentEntrySizeError) -> Self {
        Self::BasicError(BasicError::InvalidPrivateDocumentEntrySizeError(err))
    }
}
