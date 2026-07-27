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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::fee::epoch::CreditsPerEpoch;

    fn make_block_fees() -> BlockFees {
        BlockFees::from_fees(500, 300)
    }

    #[test]
    fn block_fees_wrapper_getters_delegate_correctly() {
        let fees = make_block_fees();
        assert_eq!(fees.storage_fee(), 500);
        assert_eq!(fees.processing_fee(), 300);
        assert!(fees.refunds_per_epoch().is_empty());
    }

    #[test]
    fn block_fees_wrapper_setters_delegate_correctly() {
        let mut fees = make_block_fees();
        fees.set_processing_fee(999);
        fees.set_storage_fee(888);
        assert_eq!(fees.processing_fee(), 999);
        assert_eq!(fees.storage_fee(), 888);

        let mut refunds = CreditsPerEpoch::default();
        refunds.insert(2, 100);
        fees.set_refunds_per_epoch(refunds);
        assert_eq!(*fees.refunds_per_epoch().get(&2).unwrap(), 100);
    }

    #[test]
    fn block_fees_wrapper_refunds_per_epoch_mut() {
        let mut fees = make_block_fees();
        fees.refunds_per_epoch_mut().insert(7, 42);
        assert_eq!(*fees.refunds_per_epoch().get(&7).unwrap(), 42);
    }

    #[test]
    fn block_fees_wrapper_refunds_per_epoch_owned() {
        let mut fees = make_block_fees();
        fees.refunds_per_epoch_mut().insert(3, 55);
        let owned = fees.refunds_per_epoch_owned();
        assert_eq!(*owned.get(&3).unwrap(), 55);
    }

    #[test]
    fn block_fees_from_v0_conversion() {
        let v0 = BlockFeesV0::from_fees(10, 20);
        let fees: BlockFees = v0.into();
        assert_eq!(fees.storage_fee(), 10);
        assert_eq!(fees.processing_fee(), 20);
    }
}
