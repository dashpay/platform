use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Asset lock output {output_index} is not a valid standard OP_RETURN output")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityAssetLockTransactionOutputError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    output_index: usize,
}

impl InvalidIdentityAssetLockTransactionOutputError {
    pub fn new(output_index: usize) -> Self {
        Self { output_index }
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }
}

impl From<InvalidIdentityAssetLockTransactionOutputError> for ConsensusError {
    fn from(err: InvalidIdentityAssetLockTransactionOutputError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityAssetLockTransactionOutputError(
            err,
        ))
    }
}
