use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("documents of referenced document type {document_type_name} in contract {contract_id} can be deleted; a permanentDocument reference at path {path} requires a document type with canBeDeleted: false")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentTypeDeletableError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    path: String,
}

impl ReferencedDocumentTypeDeletableError {
    pub fn new(contract_id: Identifier, document_type_name: String, path: String) -> Self {
        Self {
            contract_id,
            document_type_name,
            path,
        }
    }

    pub fn contract_id(&self) -> &Identifier {
        &self.contract_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl From<ReferencedDocumentTypeDeletableError> for ConsensusError {
    fn from(err: ReferencedDocumentTypeDeletableError) -> Self {
        Self::StateError(StateError::ReferencedDocumentTypeDeletableError(err))
    }
}
