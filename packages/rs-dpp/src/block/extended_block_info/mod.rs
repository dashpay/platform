use crate::block::block_info::BlockInfo;
use crate::block::extended_block_info::v0::{
    ExtendedBlockInfoV0, ExtendedBlockInfoV0Getters, ExtendedBlockInfoV0Setters,
};
use crate::protocol_error::ProtocolError;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::version::FeatureVersion;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};

pub mod v0;

/// Extended Block information
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    From,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[serde(tag = "$formatVersion")]
pub enum ExtendedBlockInfo {
    #[serde(rename = "0")]
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
}

impl ExtendedBlockInfoV0Getters for ExtendedBlockInfo {
    fn basic_info(&self) -> &BlockInfo {
        match self {
            ExtendedBlockInfo::V0(v0) => &v0.basic_info,
        }
    }

    fn basic_info_mut(&mut self) -> &mut BlockInfo {
        match self {
            ExtendedBlockInfo::V0(v0) => &mut v0.basic_info,
        }
    }

    fn basic_info_owned(self) -> BlockInfo {
        match self {
            ExtendedBlockInfo::V0(v0) => v0.basic_info,
        }
    }

    fn app_hash(&self) -> &[u8; 32] {
        match self {
            ExtendedBlockInfo::V0(v0) => &v0.app_hash,
        }
    }

    fn quorum_hash(&self) -> &[u8; 32] {
        match self {
            ExtendedBlockInfo::V0(v0) => &v0.quorum_hash,
        }
    }

    fn proposer_pro_tx_hash(&self) -> &[u8; 32] {
        match self {
            ExtendedBlockInfo::V0(v0) => &v0.proposer_pro_tx_hash,
        }
    }

    fn block_id_hash(&self) -> &[u8; 32] {
        match self {
            ExtendedBlockInfo::V0(v0) => &v0.block_id_hash,
        }
    }

    fn signature(&self) -> &[u8; 96] {
        match self {
            ExtendedBlockInfo::V0(v0) => &v0.signature,
        }
    }

    fn round(&self) -> u32 {
        match self {
            ExtendedBlockInfo::V0(v0) => v0.round,
        }
    }
}

impl ExtendedBlockInfoV0Setters for ExtendedBlockInfo {
    fn set_basic_info(&mut self, info: BlockInfo) {
        match self {
            ExtendedBlockInfo::V0(v0) => {
                v0.set_basic_info(info);
            }
        }
    }

    fn set_app_hash(&mut self, hash: [u8; 32]) {
        match self {
            ExtendedBlockInfo::V0(v0) => {
                v0.set_app_hash(hash);
            }
        }
    }

    fn set_quorum_hash(&mut self, hash: [u8; 32]) {
        match self {
            ExtendedBlockInfo::V0(v0) => {
                v0.set_quorum_hash(hash);
            }
        }
    }

    fn set_signature(&mut self, signature: [u8; 96]) {
        match self {
            ExtendedBlockInfo::V0(v0) => {
                v0.set_signature(signature);
            }
        }
    }

    fn set_round(&mut self, round: u32) {
        match self {
            ExtendedBlockInfo::V0(v0) => {
                v0.set_round(round);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::block_info::BlockInfo;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};

    #[test]
    fn test_extended_block_info_bincode() {
        let block_info: ExtendedBlockInfo = ExtendedBlockInfoV0 {
            basic_info: BlockInfo::default(),
            app_hash: [1; 32],
            quorum_hash: [2; 32],
            block_id_hash: [3; 32],
            proposer_pro_tx_hash: [4; 32],
            signature: [3; 96],
            round: 1,
        }
        .into();

        // Serialize into a vector
        let encoded =
            PlatformSerializable::serialize_to_bytes(&block_info).expect("expected to serialize");

        // Deserialize from the vector
        let decoded: ExtendedBlockInfo = PlatformDeserializable::deserialize_from_bytes(&encoded)
            .expect("expected to deserialize");

        assert_eq!(block_info, decoded);
    }
}

// (TODO replaced) extendedblockinfo — needs explicit fixture (no Default).

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_extendedblockinfo {
    use super::*;
    use crate::block::block_info::BlockInfo;
    use crate::block::extended_block_info::v0::ExtendedBlockInfoV0;
    use platform_value::platform_value;
    use serde_json::json;

    fn fixture() -> ExtendedBlockInfo {
        ExtendedBlockInfo::V0(ExtendedBlockInfoV0 {
            basic_info: BlockInfo::default(),
            app_hash: [0x11; 32],
            quorum_hash: [0x22; 32],
            block_id_hash: [0x33; 32],
            proposer_pro_tx_hash: [0x44; 32],
            signature: [0x55; 96],
            round: 3,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `json_safe_fields` proc-macro converts u64 -> string only when above
        // JS_MAX_SAFE_INTEGER. Default `BlockInfo` (zeros) stays numeric.
        // 32-byte arrays are emitted as base64 strings (`appHash`, etc.); the
        // 96-byte signature is also base64 (no Bytes32 path). JSON erases
        // size for `round` (u32) — value-path locks `3u32` below.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "basicInfo": {
                    "timeMs": 0,
                    "height": 0,
                    "coreHeight": 0,
                    "epoch": {"index": 0},
                },
                "appHash": "ERERERERERERERERERERERERERERERERERERERERERE=",
                "quorumHash": "IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI=",
                "blockIdHash": "MzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzM=",
                "proposerProTxHash": "REREREREREREREREREREREREREREREREREREREREREQ=",
                "signature": "VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV",
                "round": 3,
            })
        );
        let recovered = ExtendedBlockInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::Value;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `[u8; 32]` -> `Value::Bytes32`, `[u8; 96]` -> `Value::Bytes`,
        // `round` is `u32` -> `Value::U32`. `BlockInfo` fields use their
        // native typed variants (U64 / U32 / U16).
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "basicInfo": {
                    "timeMs": 0u64,
                    "height": 0u64,
                    "coreHeight": 0u32,
                    "epoch": {"index": 0u16},
                },
                "appHash": Value::Bytes32([0x11; 32]),
                "quorumHash": Value::Bytes32([0x22; 32]),
                "blockIdHash": Value::Bytes32([0x33; 32]),
                "proposerProTxHash": Value::Bytes32([0x44; 32]),
                "signature": Value::Bytes(vec![0x55; 96]),
                "round": 3u32,
            })
        );
        let recovered = ExtendedBlockInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
