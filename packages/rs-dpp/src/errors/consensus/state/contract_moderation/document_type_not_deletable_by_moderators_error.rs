use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
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
#[error(
    "Documents of type {} on contract {} can not be deleted by moderators",
    document_type_name,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct DocumentTypeNotDeletableByModeratorsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
}

impl DocumentTypeNotDeletableByModeratorsError {
    pub fn new(contract_id: Identifier, document_type_name: String) -> Self {
        Self {
            contract_id,
            document_type_name,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }
}

impl From<DocumentTypeNotDeletableByModeratorsError> for ConsensusError {
    fn from(err: DocumentTypeNotDeletableByModeratorsError) -> Self {
        Self::StateError(StateError::DocumentTypeNotDeletableByModeratorsError(err))
    }
}
