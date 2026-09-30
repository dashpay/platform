use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The document a `refersTo` lookup with a computed key found, the commitment a create
/// reveals, exists but does not meet what the reference requires of it: `minimumAgeBlocks`,
/// its recorded creation block height at least that many blocks below the height of the
/// create (a document recording none never meets it).
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
    "referenced document {document_id} for path {path} does not meet the reference's requirement {field} {required}"
)]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentRequirementNotMetError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    field: String,
    required: String,
    path: String,
}

impl ReferencedDocumentRequirementNotMetError {
    pub fn new(document_id: Identifier, field: String, required: String, path: String) -> Self {
        Self {
            document_id,
            field,
            required,
            path,
        }
    }

    /// The document the lookup found, which exists but does not meet the requirement
    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    /// The `refersTo` key of the requirement, declared beside the lookup: `minimumAgeBlocks`
    pub fn field(&self) -> &str {
        &self.field
    }

    /// The value the reference requires as the schema spells it, the number of blocks for
    /// `minimumAgeBlocks`
    pub fn required(&self) -> &str {
        &self.required
    }

    /// Where the reference is declared: the property carrying it, `$ownerId` or `$creatorId`
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl From<ReferencedDocumentRequirementNotMetError> for ConsensusError {
    fn from(err: ReferencedDocumentRequirementNotMetError) -> Self {
        Self::StateError(StateError::ReferencedDocumentRequirementNotMetError(err))
    }
}
