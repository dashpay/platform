use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn document_type(&self) -> &str {
        &self.document_type
    }

    pub fn index_name(&self) -> &str {
        &self.index_name
    }
}

impl From<DataContractInvalidIndexDefinitionUpdateError> for ConsensusError {
    fn from(err: DataContractInvalidIndexDefinitionUpdateError) -> Self {
        Self::BasicError(BasicError::DataContractInvalidIndexDefinitionUpdateError(
            err,
        ))
    }
}
