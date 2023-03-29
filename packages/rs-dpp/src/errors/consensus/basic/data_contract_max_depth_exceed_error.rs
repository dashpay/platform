use crate::consensus::basic::BasicError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("JSON Schema depth is greater than {0}")]
pub struct DataContractMaxDepthExceedError {
    max_depth: usize
}

impl DataContractMaxDepthExceedError {
    pub fn new(max_depth: usize) -> Self {
        Self {
            max_depth,
        }
    }

    pub fn max_depth(&self) -> usize {
        self.max_depth
    }
}

impl From<DataContractMaxDepthExceedError> for BasicError {
    fn from(err: DataContractMaxDepthExceedError) -> Self {
        Self::DataContractMaxDepthExceedError(err)
    }
}
