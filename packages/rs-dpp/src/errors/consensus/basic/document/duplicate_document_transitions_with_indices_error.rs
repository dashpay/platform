use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error(
    "Document transitions with duplicate unique properties: {:?}",
    references
)]
pub struct DuplicateDocumentTransitionsWithIndicesError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    references: Vec<(String, [u8; 32])>,
}

impl DuplicateDocumentTransitionsWithIndicesError {
    pub fn new(references: Vec<(String, [u8; 32])>) -> Self {
        Self { references }
    }

    pub fn references(&self) -> &Vec<(String, [u8; 32])> {
        &self.references
    }
}

impl From<DuplicateDocumentTransitionsWithIndicesError> for ConsensusError {
    fn from(err: DuplicateDocumentTransitionsWithIndicesError) -> Self {
        Self::BasicError(BasicError::DuplicateDocumentTransitionsWithIndicesError(
            err,
        ))
    }
}
