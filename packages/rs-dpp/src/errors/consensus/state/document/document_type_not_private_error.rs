use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A `PrivateDocumentCreateFromShieldedPool` names a document type that is not private
/// (protocol version 15): only a type declaring `private` has a store for entries.
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
    "Document type \"{document_type_name}\" of data contract {data_contract_id} is not private: \
     only a private document type takes entries"
)]
#[platform_serialize(unversioned)]
pub struct DocumentTypeNotPrivateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    document_type_name: String,
}

impl DocumentTypeNotPrivateError {
    pub fn new(data_contract_id: Identifier, document_type_name: String) -> Self {
        Self {
            data_contract_id,
            document_type_name,
        }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }
}

impl From<DocumentTypeNotPrivateError> for ConsensusError {
    fn from(err: DocumentTypeNotPrivateError) -> Self {
        Self::StateError(StateError::DocumentTypeNotPrivateError(err))
    }
}
