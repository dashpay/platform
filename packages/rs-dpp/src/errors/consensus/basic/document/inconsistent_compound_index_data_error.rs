use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error(
    "Unique compound index properties {:?} are partially set for {document_type}",
    index_properties
)]
pub struct InconsistentCompoundIndexDataError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    index_properties: Vec<String>,
}

impl InconsistentCompoundIndexDataError {
    pub fn new(document_type: String, index_properties: Vec<String>) -> Self {
        Self {
            document_type,
            index_properties,
        }
    }

    pub fn index_properties(&self) -> Vec<String> {
        self.index_properties.clone()
    }
    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }
}

impl From<InconsistentCompoundIndexDataError> for ConsensusError {
    fn from(err: InconsistentCompoundIndexDataError) -> Self {
        Self::BasicError(BasicError::InconsistentCompoundIndexDataError(err))
    }
}
