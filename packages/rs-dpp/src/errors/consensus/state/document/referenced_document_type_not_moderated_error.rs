use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

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
#[error("documents of referenced document type {document_type_name} in contract {contract_id} do not leave state only through a moderator's recorded removal; a moderatedDocument reference at path {path} requires a document type with canBeDeleted: false, no ttl, and moderatorAbilities.delete keeping removal records (deleteKeepsRecord not false): a permanentDocument reference is the one for a document type whose documents never leave state, and a deletableDocument reference the one for any other")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentTypeNotModeratedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    path: String,
}

impl ReferencedDocumentTypeNotModeratedError {
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

impl From<ReferencedDocumentTypeNotModeratedError> for ConsensusError {
    fn from(err: ReferencedDocumentTypeNotModeratedError) -> Self {
        Self::StateError(StateError::ReferencedDocumentTypeNotModeratedError(err))
    }
}
