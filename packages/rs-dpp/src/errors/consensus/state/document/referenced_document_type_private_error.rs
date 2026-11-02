use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A document reference of any kind (`permanentDocument`, `deletableDocument` or
/// `moderatedDocument`, with or without `findBy`, or a list element) names a private document
/// type (protocol version 15): its documents are encrypted entries an anonymous transition
/// appends, which the chain never reads, so no write could ever check the reference.
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
#[error("referenced document type {document_type_name} in contract {contract_id} is private: its documents are encrypted entries the chain never reads, so the document reference at path {path} could never be checked; a private document type can not be the target of a document reference")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentTypePrivateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    path: String,
}

impl ReferencedDocumentTypePrivateError {
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

impl From<ReferencedDocumentTypePrivateError> for ConsensusError {
    fn from(err: ReferencedDocumentTypePrivateError) -> Self {
        Self::StateError(StateError::ReferencedDocumentTypePrivateError(err))
    }
}
