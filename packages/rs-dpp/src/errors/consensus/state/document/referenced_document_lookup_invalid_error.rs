use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A `refersTo` `findBy` into a document type of another contract cannot
/// resolve there: no unique index is over exactly the properties it names, or
/// a source holds a different kind of value than the property it fills, among
/// the other rules of a `findBy`. Reported at contract registration and
/// update; a `findBy` into the declaring contract's own document type is
/// refused by the contract parse instead.
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
#[error("invalid refersTo findBy ({find_by}) declared at {path}: {reason}")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentLookupInvalidError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    path: String,
    find_by: String,
    reason: String,
}

impl ReferencedDocumentLookupInvalidError {
    pub fn new(path: String, find_by: String, reason: String) -> Self {
        Self {
            path,
            find_by,
            reason,
        }
    }

    /// The declaring property, as `documentTypeName.propertyPath`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The properties the `findBy` names, joined with ", ".
    pub fn find_by(&self) -> &str {
        &self.find_by
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
