use crate::block::epoch::{Epoch, EPOCH_0};
use crate::prelude::TimestampMillis;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub const DEFAULT_BLOCK_INFO: BlockInfo = BlockInfo {
    time_ms: 0,
    height: 0,
    core_height: 0,
    epoch: EPOCH_0,
};

// We make this immutable because it should never be changed or updated
// Extended block info however is not immutable
// @immutable
/// Block information
#[derive(Clone, Default, Debug, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block time in milliseconds
    pub time_ms: TimestampMillis,

    /// Block height
    pub height: u64,

    /// Core height
    pub core_height: u32,

    /// Current fee epoch
    pub epoch: Epoch,
}

impl BlockInfo {
    // TODO: It's not actually a genesis one. We should use just default to avoid confusion
    /// Create block info for genesis block
    pub fn genesis() -> BlockInfo {
        BlockInfo::default()
    }

    /// Create default block with specified time
    pub fn default_with_time(time_ms: u64) -> BlockInfo {
        BlockInfo {
            time_ms,
            ..Default::default()
        }
    }

    /// Create default block with specified fee epoch
    pub fn default_with_epoch(epoch: Epoch) -> BlockInfo {
        BlockInfo {
            epoch,
            ..Default::default()
        }
    }
}
