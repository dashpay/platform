pub mod v0;

use crate::block::epoch::EpochIndex;
use crate::block::extended_epoch_info::v0::{ExtendedEpochInfoV0, ExtendedEpochInfoV0Getters};
use crate::protocol_error::ProtocolError;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
use crate::serialization::ValueConvertible;
use crate::util::deserializer::ProtocolVersion;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};

/// Extended Epoch information
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[derive(
    ValueConvertible,
    Clone,
    Debug,
    PartialEq,
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
pub enum ExtendedEpochInfo {
    #[serde(rename = "0")]
    V0(ExtendedEpochInfoV0),
}

impl ExtendedEpochInfoV0Getters for ExtendedEpochInfo {
    fn index(&self) -> EpochIndex {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.index,
        }
    }

    fn first_block_time(&self) -> u64 {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.first_block_time,
        }
    }

    fn first_block_height(&self) -> u64 {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.first_block_height,
        }
    }

    fn first_core_block_height(&self) -> u32 {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.first_core_block_height,
        }
    }

    fn fee_multiplier_permille(&self) -> u64 {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.fee_multiplier_permille,
        }
    }

    fn protocol_version(&self) -> ProtocolVersion {
        match self {
            ExtendedEpochInfo::V0(v0) => v0.protocol_version,
        }
    }
}

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;

    #[test]
    fn extended_epoch_info_json_round_trip() {
        let info = ExtendedEpochInfo::V0(ExtendedEpochInfoV0 {
            index: 5,
            first_block_time: 1_700_000_000_000u64,
            first_block_height: 500_000u64,
            first_core_block_height: 800_000u32,
            fee_multiplier_permille: 1_500u64,
            protocol_version: 4,
        });

        let json = info.to_json().expect("to_json should succeed");
        assert!(json["firstBlockTime"].is_number());
        assert_eq!(json["firstBlockTime"].as_u64().unwrap(), 1700000000000);
        assert!(json["firstBlockHeight"].is_number());
        assert_eq!(json["firstBlockHeight"].as_u64().unwrap(), 500000);
        assert!(json["feeMultiplierPermille"].is_number());
        assert_eq!(json["feeMultiplierPermille"].as_u64().unwrap(), 1500);
        assert!(json["firstCoreBlockHeight"].is_number());
        assert_eq!(json["firstCoreBlockHeight"].as_u64().unwrap(), 800_000);
        assert!(json["protocolVersion"].is_number());
        assert_eq!(json["protocolVersion"].as_u64().unwrap(), 4);

        let restored = ExtendedEpochInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(info, restored);
    }

    #[test]
    fn extended_epoch_info_value_round_trip() {
        let info = ExtendedEpochInfo::V0(ExtendedEpochInfoV0 {
            index: 10,
            first_block_time: u64::MAX,
            first_block_height: 0,
            first_core_block_height: u32::MAX,
            fee_multiplier_permille: 1_000,
            protocol_version: 1,
        });

        let obj = info.to_object().expect("to_object should succeed");
        let restored = ExtendedEpochInfo::from_object(obj).expect("from_object should succeed");
        assert_eq!(info, restored);
    }
}
