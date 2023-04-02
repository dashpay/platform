use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Duplicate '{index_name}' index definition for '{document_type}' document")]
pub struct DuplicateIndexError {
    document_type: String,
    index_name: String,
}

impl DuplicateIndexError {
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

impl From<DuplicateIndexError> for ConsensusError {
    fn from(err: DuplicateIndexError) -> Self {
        Self::BasicError(BasicError::DuplicateIndexError(err))
    }
}
