use crate::block::epoch::Epoch;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Extended Block information
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ExtendedBlockInfo {
    /// Basic block info
    pub basic_info: BlockInfo,
    /// App hash
    pub app_hash: [u8; 32],
    /// Quorum hash
    pub quorum_hash: [u8; 32],
    /// Hash of block ID
    pub block_id_hash: [u8; 32],
    /// Signature
    #[serde(with = "signature_serializer")]
    pub signature: [u8; 96],
    /// Round
    pub round: u32,
}

mod signature_serializer {
    use super::*;
    use serde::de::Error;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(signature: &[u8; 96], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(signature)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 96], D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf: Vec<u8> = Deserialize::deserialize(deserializer)?;
        if buf.len() != 96 {
            return Err(Error::invalid_length(buf.len(), &"array of length 96"));
        }
        let mut arr = [0u8; 96];
        arr.copy_from_slice(&buf);
        Ok(arr)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::{de::from_reader, ser::into_writer};

    #[test]
    fn test_extended_block_info_serde() {
        let block_info = ExtendedBlockInfo {
            basic_info: BlockInfo::default(),
            app_hash: [1; 32],
            quorum_hash: [2; 32],
            block_id_hash: [3; 32],
            signature: [3; 96],
            round: 1,
        };

        // Serialize into a vector
        let mut encoded: Vec<u8> = vec![];
        into_writer(&block_info, &mut encoded).unwrap();

        // Deserialize from the vector
        let decoded: ExtendedBlockInfo = from_reader(&encoded[..]).unwrap();

        assert_eq!(block_info, decoded);
    }
}
