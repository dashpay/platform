pub mod v0;

use crate::execution::types::block_fees::v0::{
    BlockFeesV0, BlockFeesV0Getters, BlockFeesV0Methods, BlockFeesV0Setters,
};
use derive_more::From;

use dpp::fee::epoch::CreditsPerEpoch;
use serde::{Deserialize, Serialize};

/// The versioned block fees
#[derive(Serialize, Deserialize, Clone, Debug, From)]
pub enum BlockFees {
    /// Version 0
    V0(BlockFeesV0),
}

impl BlockFeesV0Getters for BlockFees {
    fn processing_fee(&self) -> u64 {
        match self {
            BlockFees::V0(v0) => v0.processing_fee(),
        }
    }

    fn storage_fee(&self) -> u64 {
        match self {
            BlockFees::V0(v0) => v0.storage_fee(),
        }
    }

    fn refunds_per_epoch(&self) -> &CreditsPerEpoch {
        match self {
            BlockFees::V0(v0) => v0.refunds_per_epoch(),
        }
    }

    fn refunds_per_epoch_owned(self) -> CreditsPerEpoch {
        match self {
            BlockFees::V0(v0) => v0.refunds_per_epoch_owned(),
        }
    }

    fn refunds_per_epoch_mut(&mut self) -> &mut CreditsPerEpoch {
        match self {
            BlockFees::V0(v0) => v0.refunds_per_epoch_mut(),
        }
    }
}

impl BlockFeesV0Setters for BlockFees {
    fn set_processing_fee(&mut self, fee: u64) {
        match self {
            BlockFees::V0(v0) => v0.set_processing_fee(fee),
        }
    }

    fn set_storage_fee(&mut self, fee: u64) {
        match self {
            BlockFees::V0(v0) => v0.set_storage_fee(fee),
        }
    }

    fn set_refunds_per_epoch(&mut self, refunds: CreditsPerEpoch) {
        match self {
            BlockFees::V0(v0) => v0.set_refunds_per_epoch(refunds),
        }
    }
}

impl BlockFeesV0Methods for BlockFees {
    fn from_fees(storage_fee: u64, processing_fee: u64) -> Self {
        BlockFees::V0(BlockFeesV0::from_fees(storage_fee, processing_fee))
    }
}
