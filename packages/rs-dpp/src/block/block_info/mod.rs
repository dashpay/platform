use crate::block::epoch::{Epoch, EPOCH_0};
use crate::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

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
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
#[ferment_macro::export]
pub struct BlockInfo {
    /// Block time in milliseconds
    pub time_ms: TimestampMillis,

    /// Block height
    pub height: BlockHeight,

    /// Core height
    pub core_height: CoreBlockHeight,

    /// Current fee epoch
    pub epoch: Epoch,
}

impl fmt::Display for BlockInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BlockInfo {{ time_ms: {}, height: {}, core_height: {}, epoch: {} }}",
            self.time_ms, self.height, self.core_height, self.epoch.index
        )
    }
}

// Implementing PartialOrd for BlockInfo based on height
impl PartialOrd for BlockInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Implementing Ord for BlockInfo based on height
impl Ord for BlockInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height)
    }
}

impl BlockInfo {
    // TODO: It's not actually a genesis one. We should use just default to avoid confusion
    /// Create block info for genesis block
    pub fn genesis() -> BlockInfo {
        BlockInfo::default()
    }

    /// Create default block with specified time
    pub fn default_with_time(time_ms: TimestampMillis) -> BlockInfo {
        BlockInfo {
            time_ms,
            ..Default::default()
        }
    }

    /// Create default block with specified height
    pub fn default_with_height(height: BlockHeight) -> BlockInfo {
        BlockInfo {
            height,
            ..Default::default()
        }
    }

    /// Create default block with specified height and time
    pub fn default_with_height_and_time(
        height: BlockHeight,
        time_ms: TimestampMillis,
    ) -> BlockInfo {
        BlockInfo {
            height,
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
