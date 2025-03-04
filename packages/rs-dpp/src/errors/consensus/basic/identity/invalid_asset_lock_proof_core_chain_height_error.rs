use crate::errors::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Asset Lock proof core chain height {proof_core_chain_locked_height:?} is higher than the current consensus core height {current_core_chain_locked_height:?}.")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InvalidAssetLockProofCoreChainHeightError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub proof_core_chain_locked_height: u32,
    pub current_core_chain_locked_height: u32,
}

impl InvalidAssetLockProofCoreChainHeightError {
    pub fn new(proof_core_chain_locked_height: u32, current_core_chain_locked_height: u32) -> Self {
        Self {
            proof_core_chain_locked_height,
            current_core_chain_locked_height,
        }
    }

    pub fn proof_core_chain_locked_height(&self) -> u32 {
        self.proof_core_chain_locked_height
    }

    pub fn current_core_chain_locked_height(&self) -> u32 {
        self.current_core_chain_locked_height
    }
}

impl From<InvalidAssetLockProofCoreChainHeightError> for ConsensusError {
    fn from(err: InvalidAssetLockProofCoreChainHeightError) -> Self {
        Self::BasicError(BasicError::InvalidAssetLockProofCoreChainHeightError(err))
    }
}
