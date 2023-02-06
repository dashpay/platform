use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Document with type {document_type} has badly constructed index '{index_name}'. Existing properties in the indices should be defined in the beginning of it.")]
pub struct DataContractInvalidIndexDefinitionUpdateError {
    document_type: String,
    index_name: String,
}

impl DataContractInvalidIndexDefinitionUpdateError {
    pub fn new(document_type: String, index_name: String) -> Self {
        Self {
            document_type,
            index_name,
        }
    }

    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }

    pub fn index_name(&self) -> String {
        self.index_name.clone()
    }
}

impl From<DataContractInvalidIndexDefinitionUpdateError> for ConsensusError {
    fn from(err: DataContractInvalidIndexDefinitionUpdateError) -> Self {
        Self::DataContractInvalidIndexDefinitionUpdateError(err)
    }
}
