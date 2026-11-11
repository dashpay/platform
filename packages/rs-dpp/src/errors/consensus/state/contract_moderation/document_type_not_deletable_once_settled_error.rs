use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The approval of a moderator's deletion of a settled document of a document type that does not
/// say who must approve one (`moderatorAbilities.deleteSettled`): past its `deleteWithin`
/// window, no moderator deletes a document of the type.
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
    "Document type {} of contract {} does not let the moderators delete a settled document: it sets no moderatorAbilities.deleteSettled",
    document_type_name,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct DocumentTypeNotDeletableOnceSettledError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
}

impl DocumentTypeNotDeletableOnceSettledError {
    pub fn new(contract_id: Identifier, document_type_name: String) -> Self {
        Self {
            contract_id,
            document_type_name,
        }
    }

    /// The moderated contract
    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    /// The document type the deletion names
    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }
}

impl From<DocumentTypeNotDeletableOnceSettledError> for ConsensusError {
    fn from(err: DocumentTypeNotDeletableOnceSettledError) -> Self {
        Self::StateError(StateError::DocumentTypeNotDeletableOnceSettledError(err))
    }
}
