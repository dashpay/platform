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
#[error("documents of referenced document type {document_type_name} in contract {contract_id} leave state only through a moderator's recorded removal; a deletableDocument reference at path {path} does not take such a document type: a moderatedDocument reference is the one for it, resolving to the document or to its removal record")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentTypeModeratedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    path: String,
}

impl ReferencedDocumentTypeModeratedError {
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

impl From<ReferencedDocumentTypeModeratedError> for ConsensusError {
    fn from(err: ReferencedDocumentTypeModeratedError) -> Self {
        Self::StateError(StateError::ReferencedDocumentTypeModeratedError(err))
    }
}
