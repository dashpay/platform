use thiserror::Error;

use crate::data_contract::DataContract;
use crate::errors::ProtocolError;

// @append_only
#[derive(Error, Debug, Clone, PartialEq)]
#[error("Data Contract doesn't define document with type {doc_type}")]
#[ferment_macro::export]
pub struct InvalidDocumentTypeError {
    pub doc_type: String,
    pub data_contract: DataContract,
}

impl InvalidDocumentTypeError {
    pub fn new(doc_type: String, data_contract: DataContract) -> Self {
        Self {
            doc_type,
            data_contract,
        }
    }

    pub fn doc_type(&self) -> String {
        self.doc_type.clone()
    }
    pub fn data_contract(&self) -> DataContract {
        self.data_contract.clone()
    }
}

impl From<InvalidDocumentTypeError> for ProtocolError {
    fn from(err: InvalidDocumentTypeError) -> Self {
        ProtocolError::InvalidDocumentTypeError(err)
    }
}
