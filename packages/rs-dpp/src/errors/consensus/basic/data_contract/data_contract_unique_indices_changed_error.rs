use crate::consensus::basic::BasicError;
use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Document with type {document_type} has updated unique index named '{index_name}'. Change of unique indices is not allowed")]
pub struct DataContractUniqueIndicesChangedError {
    document_type: String,
    index_name: String,
}

impl DataContractUniqueIndicesChangedError {
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

impl From<DataContractUniqueIndicesChangedError> for ConsensusError {
    fn from(err: DataContractUniqueIndicesChangedError) -> Self {
        Self::BasicError(BasicError::DataContractUniqueIndicesChangedError(err))
    }
}
