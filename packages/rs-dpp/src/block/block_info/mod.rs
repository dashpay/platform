use crate::block::epoch::Epoch;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

// We make this immutable because it should never be changed or updated
// Extended block info however is not immutable
// @immutable
/// Block information
#[derive(Clone, Default, Debug, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block time in milliseconds
    pub time_ms: u64,

    /// Block height
    pub height: u64,

    /// Core height
    pub core_height: u32,

    /// Current fee epoch
    pub epoch: Epoch,
}

impl BlockInfo {
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
