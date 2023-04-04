use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("JSON Schema depth is greater than {max_depth:?}")]
pub struct DataContractMaxDepthExceedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    schema_depth: usize,
    max_depth: usize,
}

impl DataContractMaxDepthExceedError {
    pub fn new(schema_depth: usize, max_depth: usize) -> Self {
        Self {
            schema_depth,
            max_depth,
        }
    }

    pub fn max_depth(&self) -> usize {
        self.max_depth
    }
    pub fn schema_depth(&self) -> usize {
        self.schema_depth
    }
}

impl From<DataContractMaxDepthExceedError> for ConsensusError {
    fn from(err: DataContractMaxDepthExceedError) -> Self {
        Self::BasicError(BasicError::DataContractMaxDepthExceedError(err))
    }
}
