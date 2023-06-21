use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use crate::block::block_info::BlockInfo;

/// Extended Block information
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ExtendedBlockInfoV0 {
    /// Basic block info
    pub basic_info: BlockInfo,
    /// App hash
    pub app_hash: [u8; 32],
    /// Signature
    pub quorum_hash: [u8; 32],
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

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::{de::from_reader, ser::into_writer};

    #[test]
    fn test_extended_block_info_v0_serde_ciborium() {
        let block_info = ExtendedBlockInfoV0 {
            basic_info: BlockInfo::default(),
            app_hash: [1; 32],
            quorum_hash: [2; 32],
            signature: [3; 96],
            round: 1,
        };

        // Serialize into a vector
        let mut encoded: Vec<u8> = vec![];
        into_writer(&block_info, &mut encoded).unwrap();

        // Deserialize from the vector
        let decoded: ExtendedBlockInfoV0 = from_reader(&encoded[..]).unwrap();

        assert_eq!(block_info, decoded);
    }
}
