use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A document supplied a string longer in UTF-8 bytes than the `maxBytes` its
/// type declares on the property (or on the items of a typed array of
/// strings). `maxLength` counts characters, which can be up to four bytes each;
/// `maxBytes` bounds the stored size.
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
#[error("Property {property} is {byte_length} bytes in UTF-8, over its maxBytes of {max_bytes}")]
#[platform_serialize(unversioned)]
pub struct DocumentPropertyMaxBytesExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    property: String,
    byte_length: u32,
    max_bytes: u16,
}

impl DocumentPropertyMaxBytesExceededError {
    pub fn new(property: String, byte_length: u32, max_bytes: u16) -> Self {
        Self {
            property,
            byte_length,
            max_bytes,
        }
    }

    /// The dotted path of the property, as the document type flattens it, with
    /// the element's index (`tags[2]`) for an item of a typed array.
    pub fn property(&self) -> &str {
        &self.property
    }

    /// The UTF-8 length of the supplied string.
    pub fn byte_length(&self) -> u32 {
        self.byte_length
    }

    /// The declared bound.
    pub fn max_bytes(&self) -> u16 {
        self.max_bytes
    }
}

impl From<DocumentPropertyMaxBytesExceededError> for ConsensusError {
    fn from(err: DocumentPropertyMaxBytesExceededError) -> Self {
        Self::BasicError(BasicError::DocumentPropertyMaxBytesExceededError(err))
    }
}
