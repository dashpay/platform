use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A replace kept a `moderatedDocument` reference whose document the contract's moderators
/// removed, and had to check a `where` entry against that document again (the referring
/// property changed, or the entry is a writer gate): the removal record the reference now
/// resolves to keeps the document's id, its owner and the fields its type lists under
/// `moderatorAbilities.deleteKeepsFields`, not the property the entry compares.
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
#[error("document {document_id} that {path} refers to was removed by the contract's moderators: the reference still holds through its removal record, but where compares the document's {referenced_property}, which the record does not keep; point {path} at a document in state, or leave the properties where reads unchanged until the document is restored")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentRemovedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    path: String,
    referenced_property: String,
}

impl ReferencedDocumentRemovedError {
    pub fn new(document_id: Identifier, path: String, referenced_property: String) -> Self {
        Self {
            document_id,
            path,
            referenced_property,
        }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn referenced_property(&self) -> &str {
        &self.referenced_property
    }
}

impl From<ReferencedDocumentRemovedError> for ConsensusError {
    fn from(err: ReferencedDocumentRemovedError) -> Self {
        Self::StateError(StateError::ReferencedDocumentRemovedError(err))
    }
}
