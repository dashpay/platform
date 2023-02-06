use thiserror::Error;

use crate::data_contract::errors::DataContractError;
use crate::identifier::Identifier;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Data Contract {data_contract_id} doesn't define document with the type {document_type}")]
pub struct InvalidDocumentTypeError {
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
        self.data_contract_id.clone()
    }
}

impl From<InvalidDocumentTypeError> for DataContractError {
    fn from(err: InvalidDocumentTypeError) -> Self {
        Self::InvalidDocumentTypeError(err)
    }
}
