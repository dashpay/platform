use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::prelude::Identifier;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A document transition of a Batch names a private document type (protocol version 15). A
/// private type's documents are fixed-size encrypted entries that only an anonymous shielded
/// transition appends, so a Batch can not create, replace, delete, transfer or trade one. The
/// Batch is refused as soon as it resolves the type, before it reads or writes a document.
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
    "Document type \"{document_type_name}\" of data contract {data_contract_id} is private: a \
     batch can not create, replace, delete, transfer or trade its documents"
)]
#[platform_serialize(unversioned)]
pub struct DocumentTypePrivateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    document_type_name: String,
}

impl DocumentTypePrivateError {
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

impl From<DocumentTypePrivateError> for ConsensusError {
    fn from(err: DocumentTypePrivateError) -> Self {
        Self::BasicError(BasicError::DocumentTypePrivateError(err))
    }
}
