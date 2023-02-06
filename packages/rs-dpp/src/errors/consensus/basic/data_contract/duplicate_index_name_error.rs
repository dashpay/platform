use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Duplicate index name '{duplicate_index_name}' defined in '{document_type}' document")]
pub struct DuplicateIndexNameError {
    document_type: String,
    duplicate_index_name: String,
}

impl DuplicateIndexNameError {
    pub fn new(document_type: String, duplicate_index_name: String) -> Self {
        Self {
            document_type,
            duplicate_index_name,
        }
    }

    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }

    pub fn duplicate_index_name(&self) -> String {
        self.duplicate_index_name.clone()
    }
}

impl From<DuplicateIndexNameError> for ConsensusError {
    fn from(err: DuplicateIndexNameError) -> Self {
        Self::DuplicateIndexNameError(err)
    }
}
