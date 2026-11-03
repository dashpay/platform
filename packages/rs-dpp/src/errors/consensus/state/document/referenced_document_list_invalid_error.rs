use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A `refersTo: listElement` whose list lives in a document type of another
/// contract cannot be served by it: the document type's documents can be
/// deleted, the list is not a stored typed array of identifiers of it, or a
/// replace could change it. Reported at contract registration and update; a
/// list in the declaring contract's own document type is refused by the
/// contract parse instead.
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
#[error("invalid refersTo listElement into inList {in_list} declared at {path}: {reason}")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentListInvalidError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    path: String,
    in_list: String,
    reason: String,
}

impl ReferencedDocumentListInvalidError {
    pub fn new(path: String, in_list: String, reason: String) -> Self {
        Self {
            path,
            in_list,
            reason,
        }
    }

    /// The declaring property, as `documentTypeName.propertyPath`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The list the declaration names, its `inList`.
    pub fn in_list(&self) -> &str {
        &self.in_list
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<ReferencedDocumentListInvalidError> for ConsensusError {
    fn from(err: ReferencedDocumentListInvalidError) -> Self {
        Self::StateError(StateError::ReferencedDocumentListInvalidError(err))
    }
}
