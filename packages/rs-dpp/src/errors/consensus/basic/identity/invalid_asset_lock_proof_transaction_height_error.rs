use crate::errors::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Core chain locked height {proof_core_chain_locked_height:?} must be higher than block {transaction_height:?} with Asset Lock transaction")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct InvalidAssetLockProofTransactionHeightError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub proof_core_chain_locked_height: u32,
    pub transaction_height: Option<u32>,
}

impl InvalidAssetLockProofTransactionHeightError {
    pub fn new(proof_core_chain_locked_height: u32, transaction_height: Option<u32>) -> Self {
        Self {
            proof_core_chain_locked_height,
            transaction_height,
        }
    }

    pub fn proof_core_chain_locked_height(&self) -> u32 {
        self.proof_core_chain_locked_height
    }

    pub fn transaction_height(&self) -> Option<u32> {
        self.transaction_height
    }
}

impl From<InvalidAssetLockProofTransactionHeightError> for ConsensusError {
    fn from(err: InvalidAssetLockProofTransactionHeightError) -> Self {
        Self::BasicError(BasicError::InvalidAssetLockProofTransactionHeightError(err))
    }
}
