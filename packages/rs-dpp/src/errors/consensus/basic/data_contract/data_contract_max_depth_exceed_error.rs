use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("JSON Schema depth is greater than {max_depth:?}")]
#[platform_serialize(unversioned)]
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
