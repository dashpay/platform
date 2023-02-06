use crate::consensus::basic::BasicError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error(
    "Unique compound index properties {:?} are partially set for {document_type}",
    index_properties
)]
pub struct InconsistentCompoundIndexDataError {
    index_properties: Vec<String>,
    document_type: String,
}

impl InconsistentCompoundIndexDataError {
    pub fn new(index_properties: Vec<String>, document_type: String) -> Self {
        Self {
            index_properties,
            document_type,
        }
    }

    pub fn index_properties(&self) -> Vec<String> {
        self.index_properties.clone()
    }
    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }
}

impl From<InconsistentCompoundIndexDataError> for BasicError {
    fn from(err: InconsistentCompoundIndexDataError) -> Self {
        Self::InconsistentCompoundIndexDataError(err)
    }
}
