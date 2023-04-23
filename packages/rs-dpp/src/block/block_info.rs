use crate::block::epoch::Epoch;
use crate::ProtocolError;
use bincode::config;
use bincode::{Decode, Encode};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use crate::serialization_traits::PlatformDeserializable;

/// Block information
#[derive(Clone, Default, Encode, Decode, PlatformSerialize, PlatformDeserialize)]
#[platform_error_type(ProtocolError)]
#[platform_deserialize_limit(15000)]
pub struct BlockInfo {
    /// Block time in milliseconds
    pub time_ms: u64,

    /// Block height
    pub height: u64,

    /// Core height
    pub core_height: u32,

    /// Current fee epoch
    pub epoch: Epoch,
    // /// current quorum
    // pub current_validator_set_quorum_hash: QuorumHash,
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
