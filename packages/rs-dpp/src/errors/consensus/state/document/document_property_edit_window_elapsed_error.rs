use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::identity::TimestampMillis;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A document replace changed, added or removed a property the document type
/// lists under its `immutableAfter` keyword (protocol version 14) after the
/// property's window had passed: block time was later than the document's
/// `$createdAt` plus the window.
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
    "property '{}' of document {} (type '{}') could be changed for {} seconds after the document was created at {}, which block time {} is past",
    property,
    document_id,
    document_type_name,
    window_seconds,
    created_at,
    block_time
)]
#[platform_serialize(unversioned)]
pub struct DocumentPropertyEditWindowElapsedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    document_type_name: String,
    property: String,
    created_at: TimestampMillis,
    window_seconds: u32,
    block_time: TimestampMillis,
}

impl DocumentPropertyEditWindowElapsedError {
    pub fn new(
        document_id: Identifier,
        document_type_name: String,
        property: String,
        created_at: TimestampMillis,
        window_seconds: u32,
        block_time: TimestampMillis,
    ) -> Self {
        Self {
            document_id,
            document_type_name,
            property,
            created_at,
            window_seconds,
            block_time,
        }
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    /// The first property (in name order) the replace tried to change after
    /// its window
    pub fn property(&self) -> &str {
        &self.property
    }

    /// The document's `$createdAt`, in milliseconds, the window's start
    pub fn created_at(&self) -> TimestampMillis {
        self.created_at
    }

    /// For how long after the document's creation the property could change
    pub fn window_seconds(&self) -> u32 {
        self.window_seconds
    }

    /// The block time the replace was judged at, in milliseconds
    pub fn block_time(&self) -> TimestampMillis {
        self.block_time
    }
}

impl From<DocumentPropertyEditWindowElapsedError> for ConsensusError {
    fn from(err: DocumentPropertyEditWindowElapsedError) -> Self {
        Self::StateError(StateError::DocumentPropertyEditWindowElapsedError(err))
    }
}
