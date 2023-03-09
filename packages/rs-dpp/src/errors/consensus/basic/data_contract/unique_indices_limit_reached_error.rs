use crate::consensus::basic::IndexError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
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

    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }
    pub fn index_limit(&self) -> usize {
        self.index_limit
    }
}

impl From<UniqueIndicesLimitReachedError> for IndexError {
    fn from(err: UniqueIndicesLimitReachedError) -> Self {
        Self::UniqueIndicesLimitReachedError(err)
    }
}
