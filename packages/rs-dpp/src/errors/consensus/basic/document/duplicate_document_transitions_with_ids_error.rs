use crate::consensus::basic::BasicError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Document transitions with duplicate IDs {:?}", references)]
pub struct DuplicateDocumentTransitionsWithIdsError {
    references: Vec<(String, Vec<u8>)>,
}

impl DuplicateDocumentTransitionsWithIdsError {
    pub fn new(references: Vec<(String, Vec<u8>)>) -> Self {
        Self { references }
    }

    pub fn references(&self) -> Vec<(String, Vec<u8>)> {
        self.references.clone()
    }
}

impl From<DuplicateDocumentTransitionsWithIdsError> for BasicError {
    fn from(err: DuplicateDocumentTransitionsWithIdsError) -> Self {
        Self::DuplicateDocumentTransitionsWithIdsError(err)
    }
}
