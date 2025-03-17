pub mod getters;

use crate::fee::Credits;
use crate::prelude::{BlockHeight, BlockHeightInterval, CoreBlockHeight, TimestampMillis};
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Finalized Epoch information
#[derive(Clone, Debug, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub struct FinalizedEpochInfoV0 {
    /// First block time
    pub first_block_time: TimestampMillis,
    /// First block height
    pub first_block_height: BlockHeight,
    /// Total blocks in epoch
    pub total_blocks_in_epoch: BlockHeightInterval,
    /// First core block height
    pub first_core_block_height: CoreBlockHeight,
    /// Last core block height
    pub next_epoch_start_core_block_height: CoreBlockHeight,
    /// Total processing fees
    pub total_processing_fees: Credits,
    /// Total distributed storage fees
    pub total_distributed_storage_fees: Credits,
    /// Total created storage fees
    pub total_created_storage_fees: Credits,
    /// Total rewards given from core subsidy
    pub core_block_rewards: Credits,
    /// Block proposers
    pub block_proposers: BTreeMap<Identifier, u64>,
    /// Fee multiplier that you would divide by 1000 to get float value
    pub fee_multiplier_permille: u64,
    /// Protocol version
    pub protocol_version: u32,
}
