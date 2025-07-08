use crate::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
use crate::fee::Credits;
use crate::prelude::{BlockHeight, BlockHeightInterval, CoreBlockHeight, TimestampMillis};
use platform_value::Identifier;
use std::collections::BTreeMap;

/// Trait for accessing fields of `FinalizedEpochInfoV0`.
pub trait FinalizedEpochInfoGettersV0 {
    /// Returns the first block time.
    fn first_block_time(&self) -> TimestampMillis;

    /// Returns the first block height.
    fn first_block_height(&self) -> BlockHeight;

    /// Returns the total blocks in the epoch.
    fn total_blocks_in_epoch(&self) -> BlockHeightInterval;

    /// Returns the first core block height.
    fn first_core_block_height(&self) -> CoreBlockHeight;

    /// Returns the last core block height.
    fn next_epoch_start_core_block_height(&self) -> CoreBlockHeight;

    /// Returns the total processing fees.
    fn total_processing_fees(&self) -> Credits;

    /// Returns the total distributed storage fees.
    fn total_distributed_storage_fees(&self) -> Credits;

    /// Returns the total created storage fees.
    fn total_created_storage_fees(&self) -> Credits;

    /// Total rewards given from core subsidy
    fn core_block_rewards(&self) -> Credits;

    /// Returns a reference to the block proposers map.
    fn block_proposers(&self) -> &BTreeMap<Identifier, u64>;

    /// Returns the fee multiplier (permille).
    fn fee_multiplier_permille(&self) -> u64;

    /// Returns the protocol version.
    fn protocol_version(&self) -> u32;
}

impl FinalizedEpochInfoGettersV0 for FinalizedEpochInfoV0 {
    fn first_block_time(&self) -> TimestampMillis {
        self.first_block_time
    }

    fn first_block_height(&self) -> BlockHeight {
        self.first_block_height
    }

    fn total_blocks_in_epoch(&self) -> BlockHeightInterval {
        self.total_blocks_in_epoch
    }

    fn first_core_block_height(&self) -> CoreBlockHeight {
        self.first_core_block_height
    }

    fn next_epoch_start_core_block_height(&self) -> CoreBlockHeight {
        self.next_epoch_start_core_block_height
    }

    fn total_processing_fees(&self) -> Credits {
        self.total_processing_fees
    }

    fn total_distributed_storage_fees(&self) -> Credits {
        self.total_distributed_storage_fees
    }

    fn total_created_storage_fees(&self) -> Credits {
        self.total_created_storage_fees
    }

    fn core_block_rewards(&self) -> Credits {
        self.core_block_rewards
    }

    fn block_proposers(&self) -> &BTreeMap<Identifier, u64> {
        &self.block_proposers
    }

    fn fee_multiplier_permille(&self) -> u64 {
        self.fee_multiplier_permille
    }

    fn protocol_version(&self) -> u32 {
        self.protocol_version
    }
}
