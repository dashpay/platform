use crate::consensus::basic::{BasicError, IndexError};
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::data_contract::document_type::Index;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("All or none of unique compound properties must be set for '{document_type}' document")]
pub struct InvalidCompoundIndexError {
    document_type: String,
    index_definition: Index,
}

impl InvalidCompoundIndexError {
    pub fn new(document_type: String, index_definition: Index) -> Self {
        Self {
            document_type,
            index_definition,
        }
    }

    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }
    pub fn index_definition(&self) -> Index {
        self.index_definition.clone()
    }
}

impl From<InvalidCompoundIndexError> for ConsensusError {
    fn from(err: InvalidCompoundIndexError) -> Self {
        Self::BasicError(Box::new(BasicError::IndexError(
            IndexError::InvalidCompoundIndexError(err),
        )))
    }
}
