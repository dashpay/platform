use crate::consensus::basic::BasicError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error(
    "Document transitions with duplicate unique properties: {:?}",
    references
)]
pub struct DuplicateDocumentTransitionsWithIndicesError {
    references: Vec<(String, [u8; 32])>,
}

impl DuplicateDocumentTransitionsWithIndicesError {
    pub fn new(references: Vec<(String, [u8; 32])>) -> Self {
        Self { references }
    }

    pub fn references(&self) -> Vec<(String, [u8; 32])> {
        self.references.clone()
    }
}

impl From<DuplicateDocumentTransitionsWithIndicesError> for BasicError {
    fn from(err: DuplicateDocumentTransitionsWithIndicesError) -> Self {
        Self::DuplicateDocumentTransitionsWithIndicesError(err)
    }
}
