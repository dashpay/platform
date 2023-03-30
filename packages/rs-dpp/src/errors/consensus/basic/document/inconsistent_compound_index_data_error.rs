use crate::consensus::basic::BasicError;
use thiserror::Error;
use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error(
    "Unique compound index properties {:?} are partially set for {document_type}",
    index_properties
)]
pub struct InconsistentCompoundIndexDataError {
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
