use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Asset Lock output {output_index} has invalid public key hash. Must be 20 length bytes hash"
)]
#[platform_serialize(unversioned)]
pub struct InvalidAssetLockTransactionOutputReturnSizeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    output_index: usize,
}

impl InvalidAssetLockTransactionOutputReturnSizeError {
    pub fn new(output_index: usize) -> Self {
        Self { output_index }
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }
}

impl From<InvalidAssetLockTransactionOutputReturnSizeError> for ConsensusError {
    fn from(err: InvalidAssetLockTransactionOutputReturnSizeError) -> Self {
        Self::BasicError(BasicError::InvalidAssetLockTransactionOutputReturnSizeError(err))
    }
}
