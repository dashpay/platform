use crate::block::epoch::Epoch;
use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};
use crate::ProtocolError;
use bincode::config;
use bincode::{Decode, Encode};

use platform_serialization::{PlatformDeserialize, PlatformSerialize};

/// Extended Block information
#[derive(Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize)]
#[platform_error_type(ProtocolError)]
#[platform_deserialize_limit(15000)]
pub struct ExtendedBlockInfo {
    /// Version byte (in case we need to extend this later)
    pub version: u16,
    /// Basic block info
    pub basic_info: BlockInfo,
    /// Signature
    pub quorum_hash: [u8; 32],
    /// Signature
    pub signature: [u8; 96],
    /// Round
    pub round: u32,
}

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
