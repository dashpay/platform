use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::prelude::Identifier;

use bincode::{Decode, Encode};
use platform_value::Value;

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document type {document_type_name} is removed. Removing document types from existing contracts is not supported")]
#[platform_serialize(unversioned)]
pub struct DocumentTypeRemovedError {
    data_contract_id: Identifier,
    document_type_name: String,
}

impl DocumentTypeRemovedError {
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

impl From<DocumentTypeRemovedError> for ConsensusError {
    fn from(err: DocumentTypeRemovedError) -> Self {
        Self::BasicError(BasicError::DocumentTypeRemovedError(err))
    }
}
