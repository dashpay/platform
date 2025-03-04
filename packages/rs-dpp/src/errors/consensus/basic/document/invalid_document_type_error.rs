use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data Contract {data_contract_id} doesn't define document with the type {document_type}")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InvalidDocumentTypeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub document_type: String,
    pub data_contract_id: Identifier,
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
