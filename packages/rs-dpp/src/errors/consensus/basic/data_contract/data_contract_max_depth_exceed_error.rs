use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("JSON Schema depth is greater than {max_depth:?}")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct DataContractMaxDepthExceedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub max_depth: usize,
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
