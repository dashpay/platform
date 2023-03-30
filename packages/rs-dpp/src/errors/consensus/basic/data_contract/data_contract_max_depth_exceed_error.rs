use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("JSON Schema depth is greater than {max_depth:?}")]
pub struct DataContractMaxDepthExceedError {
    max_depth: usize,
}

impl DataContractMaxDepthExceedError {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    pub fn max_depth(&self) -> usize {
        self.max_depth
    }
}

impl From<DataContractMaxDepthExceedError> for ConsensusError {
    fn from(err: DataContractMaxDepthExceedError) -> Self {
        Self::BasicError(BasicError::DataContractMaxDepthExceedError(err))
    }
}
