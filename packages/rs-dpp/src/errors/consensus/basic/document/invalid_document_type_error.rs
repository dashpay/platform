use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::prelude::Identifier;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data Contract {data_contract_id} doesn't define document with the type {document_type}")]
#[platform_serialize(unversioned)]
pub struct InvalidDocumentTypeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    data_contract_id: Identifier,
}

impl InvalidDocumentTypeError {
    pub fn new(document_type: String, data_contract_id: Identifier) -> Self {
        Self {
            document_type,
            data_contract_id,
        }
    }

    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }
}

impl From<InvalidDocumentTypeError> for ConsensusError {
    fn from(err: InvalidDocumentTypeError) -> Self {
        Self::BasicError(BasicError::InvalidDocumentTypeError(err))
    }
}
