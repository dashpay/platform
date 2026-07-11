use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, PartialEq,
)]
#[error("Asset lock transaction has too many inputs: {actual_inputs} (max {max_inputs}). Consolidate UTXOs before creating the asset lock.")]
#[platform_serialize(unversioned)]
pub struct IdentityAssetLockTransactionTooManyInputsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    max_inputs: u16,
    actual_inputs: u16,
}

impl IdentityAssetLockTransactionTooManyInputsError {
    pub fn new(actual_inputs: u16, max_inputs: u16) -> Self {
        Self {
            max_inputs,
            actual_inputs,
        }
    }

    pub fn max_inputs(&self) -> u16 {
        self.max_inputs
    }

    pub fn actual_inputs(&self) -> u16 {
        self.actual_inputs
    }
}

impl From<IdentityAssetLockTransactionTooManyInputsError> for ConsensusError {
    fn from(err: IdentityAssetLockTransactionTooManyInputsError) -> Self {
        Self::BasicError(BasicError::IdentityAssetLockTransactionTooManyInputsError(
            err,
        ))
    }
}
