use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Asset Lock Transaction Output with index {output_index} not found")]
#[platform_serialize(unversioned)]
pub struct IdentityAssetLockTransactionOutputNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    output_index: usize,
}

impl IdentityAssetLockTransactionOutputNotFoundError {
    pub fn new(output_index: usize) -> Self {
        Self { output_index }
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }
}

impl From<IdentityAssetLockTransactionOutputNotFoundError> for ConsensusError {
    fn from(err: IdentityAssetLockTransactionOutputNotFoundError) -> Self {
        Self::BasicError(BasicError::IdentityAssetLockTransactionOutputNotFoundError(
            err,
        ))
    }
}
