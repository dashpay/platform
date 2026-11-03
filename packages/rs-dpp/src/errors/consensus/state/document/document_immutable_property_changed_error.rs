use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A document replace changed, added or removed a property the document type
/// lists under its `immutable` keyword (protocol version 14). Only the
/// properties outside that list may change between revisions.
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
#[error("property '{property}' of document {document_id} (type '{document_type_name}') is immutable and cannot be changed by a replace")]
#[platform_serialize(unversioned)]
pub struct DocumentImmutablePropertyChangedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    document_type_name: String,
    property: String,
}

impl DocumentImmutablePropertyChangedError {
    pub fn new(document_id: Identifier, document_type_name: String, property: String) -> Self {
        Self {
            document_id,
            document_type_name,
            property,
        }
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    /// The first immutable property (in name order) the replace tried to
    /// change.
    pub fn property(&self) -> &str {
        &self.property
    }
}

impl From<DocumentImmutablePropertyChangedError> for ConsensusError {
    fn from(err: DocumentImmutablePropertyChangedError) -> Self {
        Self::StateError(StateError::DocumentImmutablePropertyChangedError(err))
    }
}
