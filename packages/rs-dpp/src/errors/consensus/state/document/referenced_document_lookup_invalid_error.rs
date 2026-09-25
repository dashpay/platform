use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A `refersTo` lookup into a document type of another contract cannot resolve
/// there: the named index is missing or not unique, its keys do not cover the
/// index exactly, or a source holds a different kind of value than its index
/// property. Reported at contract registration and update; a lookup into the
/// declaring contract's own document type is refused by the contract parse
/// instead.
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
#[error("invalid refersTo lookup through index {index} declared at {path}: {reason}")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentLookupInvalidError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    path: String,
    index: String,
    reason: String,
}

impl ReferencedDocumentLookupInvalidError {
    pub fn new(path: String, index: String, reason: String) -> Self {
        Self {
            path,
            index,
            reason,
        }
    }

    /// The declaring property, as `documentTypeName.propertyPath`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The index the lookup names.
    pub fn index(&self) -> &str {
        &self.index
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<ReferencedDocumentLookupInvalidError> for ConsensusError {
    fn from(err: ReferencedDocumentLookupInvalidError) -> Self {
        Self::StateError(StateError::ReferencedDocumentLookupInvalidError(err))
    }
}
