use crate::block::epoch::Epoch;
use crate::ProtocolError;
use bincode::config;
use bincode::{Decode, Encode};

/// Block information
#[derive(Clone, Default, Encode, Decode)]
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

    /// Serialize block info
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        let config = config::standard().with_big_endian().with_no_limit();
        bincode::encode_to_vec(self, config).map_err(|e| {
            ProtocolError::EncodingError(format!("unable to serialize data contract {e}"))
        })
    }

    /// The size of serialized block info
    pub fn serialized_size(&self) -> Result<usize, ProtocolError> {
        self.serialize().map(|a| a.len())
    }

    /// Deserialization of block info
    pub fn deserialize(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let config = config::standard().with_big_endian().with_limit::<15000>();
        bincode::decode_from_slice(bytes, config)
            .map_err(|e| {
                ProtocolError::EncodingError(format!("unable to deserialize block info {}", e))
            })
            .map(|(a, _)| a)
    }
}
