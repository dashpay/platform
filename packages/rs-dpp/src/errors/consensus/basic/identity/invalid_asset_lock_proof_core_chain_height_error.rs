use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset Lock proof core chain height {proof_core_chain_locked_height:?} is higher than the current consensus core height {current_core_chain_locked_height:?}.")]
pub struct InvalidAssetLockProofCoreChainHeightError {
    proof_core_chain_locked_height: u32,
    current_core_chain_locked_height: u32,
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
        Self::InvalidAssetLockProofCoreChainHeightError(err)
    }
}
