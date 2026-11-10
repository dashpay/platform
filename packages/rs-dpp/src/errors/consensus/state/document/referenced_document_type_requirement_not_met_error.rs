use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A document reference naming its target through the referring document
/// (`contractIdProperty`, `documentTypeProperty`, protocol version 15) points at a document
/// type that does not meet the reference's `documentTypeRequirements`: no choice of its
/// properties fills the slots `require` asks for.
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
#[error("referenced document type {document_type_name} in contract {contract_id} does not meet the documentTypeRequirements of the document reference at path {path}: its properties do not fill the slots the reference requires")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentTypeRequirementNotMetError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    path: String,
}

impl ReferencedDocumentTypeRequirementNotMetError {
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

impl From<ReferencedDocumentTypeRequirementNotMetError> for ConsensusError {
    fn from(err: ReferencedDocumentTypeRequirementNotMetError) -> Self {
        Self::StateError(StateError::ReferencedDocumentTypeRequirementNotMetError(
            err,
        ))
    }
}
