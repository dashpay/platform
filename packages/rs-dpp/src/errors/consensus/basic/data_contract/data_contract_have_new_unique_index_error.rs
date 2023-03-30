use crate::consensus::basic::BasicError;
use thiserror::Error;
use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Document with type {document_type} has a new unique index named '{index_name}'. Adding unique indices during Data Contract update is not allowed.")]
pub struct DataContractHaveNewUniqueIndexError {
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

    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }

    pub fn index_name(&self) -> String {
        self.index_name.clone()
    }
}

impl From<DataContractHaveNewUniqueIndexError> for ConsensusError {
    fn from(err: DataContractHaveNewUniqueIndexError) -> Self {
        Self::BasicError(BasicError::DataContractHaveNewUniqueIndexError(err))
    }
}
