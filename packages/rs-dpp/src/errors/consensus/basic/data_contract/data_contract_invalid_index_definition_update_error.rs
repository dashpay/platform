use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Document with type {document_type} has badly constructed index '{index_path}'. Existing properties in the indices should be defined in the beginning of it.")]
pub struct DataContractInvalidIndexDefinitionUpdateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    index_path: String,
}

impl DataContractInvalidIndexDefinitionUpdateError {
    pub fn new(document_type: String, index_name: String) -> Self {
        Self {
            document_type,
            index_path: index_name,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }

    pub fn index_path(&self) -> &str {
        &self.index_path
    }
}

impl From<DataContractInvalidIndexDefinitionUpdateError> for ConsensusError {
    fn from(err: DataContractInvalidIndexDefinitionUpdateError) -> Self {
        Self::BasicError(BasicError::DataContractInvalidIndexDefinitionUpdateError(
            err,
        ))
    }
}
