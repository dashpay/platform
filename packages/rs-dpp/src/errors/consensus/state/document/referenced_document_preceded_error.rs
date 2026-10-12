use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The commitment a `refersTo` lookup found for a create is preceded by another identity's: a
/// document of the same type holding the same value at the property the reference's
/// `chooseFirst` groups by (`among`), ordered before it by the `chooseFirst`'s `by`, not past
/// its `ttl`, and valid, holding the `validWhen` function computed with its own owner. The
/// first valid commitment in that order wins (the one from the earliest block, ordered by
/// `$createdAtBlockHeight` ascending), so the create is refused.
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
    "referenced document {document_id} for path {path} is preceded in its chooseFirst order by document {preceding_document_id}, a valid commitment of another identity holding the same {property}"
)]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentPrecededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    preceding_document_id: Identifier,
    property: String,
    path: String,
}

impl ReferencedDocumentPrecededError {
    pub fn new(
        document_id: Identifier,
        preceding_document_id: Identifier,
        property: String,
        path: String,
    ) -> Self {
        Self {
            document_id,
            preceding_document_id,
            property,
            path,
        }
    }

    /// The document the lookup found, the writer's own commitment
    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    /// The valid commitment of another identity that comes before it in the `chooseFirst` order
    pub fn preceding_document_id(&self) -> &Identifier {
        &self.preceding_document_id
    }

    /// The property of the referenced document type the reference's `chooseFirst` groups by
    /// (`among`), which both commitments hold the same value at
    pub fn property(&self) -> &str {
        &self.property
    }

    /// Where the reference is declared: the property carrying it, `$ownerId` or `$creatorId`
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl From<ReferencedDocumentPrecededError> for ConsensusError {
    fn from(err: ReferencedDocumentPrecededError) -> Self {
        Self::StateError(StateError::ReferencedDocumentPrecededError(err))
    }
}
