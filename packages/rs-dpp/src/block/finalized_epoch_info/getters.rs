use crate::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
use crate::block::finalized_epoch_info::FinalizedEpochInfo;
use crate::fee::Credits;
use crate::prelude::{BlockHeight, BlockHeightInterval, CoreBlockHeight, TimestampMillis};
use platform_value::Identifier;
use std::collections::BTreeMap;

impl FinalizedEpochInfoGettersV0 for FinalizedEpochInfo {
    fn first_block_time(&self) -> TimestampMillis {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.first_block_time(),
        }
    }

    fn first_block_height(&self) -> BlockHeight {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.first_block_height(),
        }
    }

    fn total_blocks_in_epoch(&self) -> BlockHeightInterval {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.total_blocks_in_epoch(),
        }
    }

    fn first_core_block_height(&self) -> CoreBlockHeight {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.first_core_block_height(),
        }
    }

    fn next_epoch_start_core_block_height(&self) -> CoreBlockHeight {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.next_epoch_start_core_block_height(),
        }
    }

    fn total_processing_fees(&self) -> Credits {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.total_processing_fees(),
        }
    }

    fn total_distributed_storage_fees(&self) -> Credits {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.total_distributed_storage_fees(),
        }
    }

    fn total_created_storage_fees(&self) -> Credits {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.total_created_storage_fees(),
        }
    }

    fn core_block_rewards(&self) -> Credits {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.core_block_rewards(),
        }
    }

    fn block_proposers(&self) -> &BTreeMap<Identifier, u64> {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.block_proposers(),
        }
    }

    fn fee_multiplier_permille(&self) -> u64 {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.fee_multiplier_permille(),
        }
    }

    fn protocol_version(&self) -> u32 {
        match self {
            FinalizedEpochInfo::V0(v0) => v0.protocol_version(),
        }
    }
}
