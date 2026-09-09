pub mod v0;

use crate::block::epoch::EpochIndex;
use crate::block::extended_epoch_info::v0::{ExtendedEpochInfoV0, ExtendedEpochInfoV0Getters};
use crate::protocol_error::ProtocolError;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::util::deserializer::ProtocolVersion;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};

/// Extended Epoch information
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(
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

// (TODO replaced) extendedepochinfo — needs explicit fixture (no Default).

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_extendedepochinfo {
    use super::*;
    use crate::block::extended_epoch_info::v0::ExtendedEpochInfoV0;
    use platform_value::platform_value;
    use serde_json::json;

    fn fixture() -> ExtendedEpochInfo {
        ExtendedEpochInfo::V0(ExtendedEpochInfoV0 {
            index: 7,
            first_block_time: 1_700_000_000_000,
            first_block_height: 100,
            first_core_block_height: 50,
            fee_multiplier_permille: 1500,
            protocol_version: 9,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `json_safe_fields` wraps u64 values above JS_MAX_SAFE_INTEGER as
        // strings. 1_700_000_000_000 is below the threshold (~9.0e15), so it
        // stays numeric. JSON erases u16/u32/u64 size — the value-path
        // assertion below uses explicit suffixes.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "index": 7,
                "firstBlockTime": 1_700_000_000_000_u64,
                "firstBlockHeight": 100,
                "firstCoreBlockHeight": 50,
                "feeMultiplierPermille": 1500,
                "protocolVersion": 9,
            })
        );
        let recovered = ExtendedEpochInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Source field types: index u16, first_block_time u64, first_block_height u64,
        // first_core_block_height u32, fee_multiplier_permille u64, protocol_version u32.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "index": 7u16,
                "firstBlockTime": 1_700_000_000_000_u64,
                "firstBlockHeight": 100u64,
                "firstCoreBlockHeight": 50u32,
                "feeMultiplierPermille": 1500u64,
                "protocolVersion": 9u32,
            })
        );
        let recovered = ExtendedEpochInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
