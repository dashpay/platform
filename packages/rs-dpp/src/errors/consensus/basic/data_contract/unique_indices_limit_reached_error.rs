use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// TODO JS constructor doesn't have index_limit
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("'{document_type}' document has more than '{index_limit}' unique indexes")]
pub struct UniqueIndicesLimitReachedError {
    document_type: String,
    index_limit: usize,
}

impl UniqueIndicesLimitReachedError {
    pub fn new(document_type: String, index_limit: usize) -> Self {
        Self {
            document_type,
            index_limit,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }
    pub fn index_limit(&self) -> usize {
        self.index_limit
    }
}

impl From<UniqueIndicesLimitReachedError> for ConsensusError {
    fn from(err: UniqueIndicesLimitReachedError) -> Self {
        Self::BasicError(BasicError::UniqueIndicesLimitReachedError(err))
    }
}
