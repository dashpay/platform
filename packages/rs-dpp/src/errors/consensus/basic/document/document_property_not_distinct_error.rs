use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A `distinctFrom` identifier property of the written document equals the value it must
/// differ from: the document's `$ownerId`, or the named property of the same document.
///
/// A pure structure check on document create and replace (protocol version 14): it reads
/// the transition alone, so it is a basic error, not a state one.
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
    "Document type \"{document_type_name}\" property \"{property}\" must differ from \
     \"{distinct_from}\", but the two values are equal"
)]
#[platform_serialize(unversioned)]
pub struct DocumentPropertyNotDistinctError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    /// Dotted path of the declaring property within the document type.
    property: String,
    /// What the declaration named: `$ownerId` or the dotted path of the property whose
    /// value collided.
    distinct_from: String,
}

impl DocumentPropertyNotDistinctError {
    pub fn new(document_type_name: String, property: String, distinct_from: String) -> Self {
        Self {
            document_type_name,
            property,
            distinct_from,
        }
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn property(&self) -> &str {
        &self.property
    }

    pub fn distinct_from(&self) -> &str {
        &self.distinct_from
    }
}

impl From<DocumentPropertyNotDistinctError> for ConsensusError {
    fn from(err: DocumentPropertyNotDistinctError) -> Self {
        Self::BasicError(BasicError::DocumentPropertyNotDistinctError(err))
    }
}
