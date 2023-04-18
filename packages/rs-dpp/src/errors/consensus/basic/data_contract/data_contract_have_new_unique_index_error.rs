use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Document with type {document_type} has a new unique index named '{index_name}'. Adding unique indices during Data Contract update is not allowed.")]
pub struct DataContractHaveNewUniqueIndexError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    index_name: String,
}

impl DataContractHaveNewUniqueIndexError {
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

impl From<DataContractHaveNewUniqueIndexError> for ConsensusError {
    fn from(err: DataContractHaveNewUniqueIndexError) -> Self {
        Self::BasicError(BasicError::DataContractHaveNewUniqueIndexError(err))
    }
}
