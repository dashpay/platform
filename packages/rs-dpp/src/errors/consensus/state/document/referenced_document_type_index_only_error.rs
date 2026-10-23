use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A document reference resolved by a document's id (`permanentDocument`, `deletableDocument`
/// or `moderatedDocument` without `findBy`, or a list element whose list's document is read by
/// its id) names an indexOnly document type: its documents exist only as index entries, with no
/// stored document a fetch by id can read, so no write could ever check the reference.
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
#[error("referenced document type {document_type_name} in contract {contract_id} is indexOnly: its documents exist only as index entries and can not be fetched by id, so the document reference at path {path} could never be checked; an indexOnly document type can not be the target of a document reference")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentTypeIndexOnlyError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    path: String,
}

impl ReferencedDocumentTypeIndexOnlyError {
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

impl From<ReferencedDocumentTypeIndexOnlyError> for ConsensusError {
    fn from(err: ReferencedDocumentTypeIndexOnlyError) -> Self {
        Self::StateError(StateError::ReferencedDocumentTypeIndexOnlyError(err))
    }
}
