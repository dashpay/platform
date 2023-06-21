use derive_more::From;
use crate::block::extended_block_info::v0::ExtendedBlockInfoV0;
use bincode::{Decode, Encode, config};
use serde::{Deserialize, Serialize};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use crate::block::block_info::BlockInfo;
use crate::serialization_traits::PlatformDeserializable;
use crate::serialization_traits::PlatformSerializable;
use crate::protocol_error::ProtocolError;
use crate::version::FeatureVersion;

pub mod v0;

/// Extended Block information
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, Serialize, Deserialize, PlatformSerialize, PlatformDeserialize, From)]
#[platform_error_type(ProtocolError)]
pub enum ExtendedBlockInfo {
    V0(ExtendedBlockInfoV0),
}

impl ExtendedBlockInfo {
    /// Returns the version of this ExtendedBlockInfo.
    /// Currently, the only available version is 0.
    pub fn version(&self) -> FeatureVersion {
        match self {
            ExtendedBlockInfo::V0(_) => 0,
        }
    }

    /// Returns the basic_info from the version of ExtendedBlockInfo.
    /// For V0, it returns the basic_info from ExtendedBlockInfoV0.
    pub fn basic_info(&self) -> &BlockInfo {
        match self {
            ExtendedBlockInfo::V0(v0) => &v0.basic_info,
        }
    }

    /// Returns the app_hash from the version of ExtendedBlockInfo.
    /// For V0, it returns the app_hash from ExtendedBlockInfoV0.
    pub fn app_hash(&self) -> &[u8; 32] {
        match self {
            ExtendedBlockInfo::V0(v0) => &v0.app_hash,
        }
    }

    /// Returns the quorum_hash from the version of ExtendedBlockInfo.
    /// For V0, it returns the quorum_hash from ExtendedBlockInfoV0.
    pub fn quorum_hash(&self) -> &[u8; 32] {
        match self {
            ExtendedBlockInfo::V0(v0) => &v0.quorum_hash,
        }
    }

    /// Returns the signature from the version of ExtendedBlockInfo.
    /// For V0, it returns the signature from ExtendedBlockInfoV0.
    pub fn signature(&self) -> &[u8; 96] {
        match self {
            ExtendedBlockInfo::V0(v0) => &v0.signature,
        }
    }

    /// Returns the round from the version of ExtendedBlockInfo.
    /// For V0, it returns the round from ExtendedBlockInfoV0.
    pub fn round(&self) -> u32 {
        match self {
            ExtendedBlockInfo::V0(v0) => v0.round,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::{de::from_reader, ser::into_writer};
    use crate::block::block_info::BlockInfo;

    #[test]
    fn test_extended_block_info_serde_ciborium() {
        let block_info : ExtendedBlockInfo = ExtendedBlockInfoV0 {
            basic_info: BlockInfo::default(),
            app_hash: [1; 32],
            quorum_hash: [2; 32],
            signature: [3; 96],
            round: 1,
        }.into();

        // Serialize into a vector
        let mut encoded: Vec<u8> = vec![];
        into_writer(&block_info, &mut encoded).unwrap();

        // Deserialize from the vector
        let decoded: ExtendedBlockInfo = from_reader(&encoded[..]).unwrap();

        assert_eq!(block_info, decoded);
    }

    #[test]
    fn test_extended_block_info_bincode() {
        let block_info : ExtendedBlockInfo = ExtendedBlockInfoV0 {
            basic_info: BlockInfo::default(),
            app_hash: [1; 32],
            quorum_hash: [2; 32],
            signature: [3; 96],
            round: 1,
        }.into();

        // Serialize into a vector
        let mut encoded: Vec<u8> = block_info.serialize().expect("expected to serialize");

        // Deserialize from the vector
        let decoded: ExtendedBlockInfo = ExtendedBlockInfo::deserialize(encoded);

        assert_eq!(block_info, decoded);
    }
}
