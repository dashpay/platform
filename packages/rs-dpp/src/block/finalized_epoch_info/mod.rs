mod getters;
pub mod v0;

use crate::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
use crate::protocol_error::ProtocolError;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};

/// Finalized Epoch information
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
pub enum FinalizedEpochInfo {
    #[serde(rename = "0")]
    V0(FinalizedEpochInfoV0),
}

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    #[test]
    fn finalized_epoch_info_json_round_trip() {
        let proposer_id = Identifier::from([1u8; 32]);
        let mut block_proposers = BTreeMap::new();
        block_proposers.insert(proposer_id, 42u64);

        let info = FinalizedEpochInfo::V0(FinalizedEpochInfoV0 {
            first_block_time: 1_700_000_000_000u64,
            first_block_height: 100_000u64,
            total_blocks_in_epoch: 2_000u64,
            first_core_block_height: 500_000u32,
            next_epoch_start_core_block_height: 500_200u32,
            total_processing_fees: 1_000_000u64,
            total_distributed_storage_fees: 500_000u64,
            total_created_storage_fees: 600_000u64,
            core_block_rewards: 10_000_000u64,
            block_proposers,
            fee_multiplier_permille: 1_000u64,
            protocol_version: 3,
        });

        let json = info.to_json().expect("to_json should succeed");
        assert!(json["firstBlockTime"].is_number());
        assert!(json["firstBlockHeight"].is_number());
        assert!(json["totalBlocksInEpoch"].is_number());
        assert!(json["totalProcessingFees"].is_number());
        assert!(json["feeMultiplierPermille"].is_number());
        assert!(json["firstCoreBlockHeight"].is_number());
        assert!(json["nextEpochStartCoreBlockHeight"].is_number());

        let proposers = json["blockProposers"]
            .as_object()
            .expect("blockProposers should be an object");
        assert_eq!(proposers.len(), 1);

        let expected_base58 =
            proposer_id.to_string(platform_value::string_encoding::Encoding::Base58);
        assert!(
            proposers.contains_key(&expected_base58),
            "Expected key {} in blockProposers, got keys: {:?}",
            expected_base58,
            proposers.keys().collect::<Vec<_>>()
        );

        let value = &proposers[&expected_base58];
        assert!(value.is_number());
        assert_eq!(value.as_u64().unwrap(), 42);

        let restored = FinalizedEpochInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(info, restored);
    }

    #[test]
    fn finalized_epoch_info_value_round_trip_empty_proposers() {
        let info = FinalizedEpochInfo::V0(FinalizedEpochInfoV0 {
            first_block_time: 1_000_000u64,
            first_block_height: 10_000u64,
            total_blocks_in_epoch: 500u64,
            first_core_block_height: 50_000u32,
            next_epoch_start_core_block_height: 50_200u32,
            total_processing_fees: 100u64,
            total_distributed_storage_fees: 50u64,
            total_created_storage_fees: 60u64,
            core_block_rewards: 1_000u64,
            block_proposers: BTreeMap::new(),
            fee_multiplier_permille: 1_000u64,
            protocol_version: 2,
        });

        let obj = info.to_object().expect("to_object should succeed");
        let restored = FinalizedEpochInfo::from_object(obj).expect("from_object should succeed");
        assert_eq!(info, restored);
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_finalizedepochinfo {
    use super::*;
    use crate::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    fn fixture() -> FinalizedEpochInfo {
        let mut block_proposers = BTreeMap::new();
        block_proposers.insert(Identifier::new([0xab; 32]), 5u64);
        FinalizedEpochInfo::V0(FinalizedEpochInfoV0 {
            first_block_time: 1_700_000_000_000,
            first_block_height: 100,
            total_blocks_in_epoch: 250,
            first_core_block_height: 50,
            next_epoch_start_core_block_height: 75,
            total_processing_fees: 1_000_000,
            total_distributed_storage_fees: 200_000,
            total_created_storage_fees: 250_000,
            core_block_rewards: 500_000,
            block_proposers,
            fee_multiplier_permille: 1500,
            protocol_version: 9,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `block_proposers` is `BTreeMap<Identifier, u64>` with the
        // `json_safe_identifier_u64_map` serde adapter: in JSON HR mode, keys
        // become base58-encoded `Identifier` strings and values stay numeric
        // (or string for u64 above JS_MAX_SAFE_INTEGER; 5 is well below).
        // All Credits / Heights are u64/u32 — JSON erases size; the value-path
        // assertion uses explicit suffixes to lock the typed variants.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "firstBlockTime": 1_700_000_000_000_u64,
                "firstBlockHeight": 100,
                "totalBlocksInEpoch": 250,
                "firstCoreBlockHeight": 50,
                "nextEpochStartCoreBlockHeight": 75,
                "totalProcessingFees": 1_000_000,
                "totalDistributedStorageFees": 200_000,
                "totalCreatedStorageFees": 250_000,
                "coreBlockRewards": 500_000,
                "blockProposers": {
                    "CZ8YUVdk7znjrUmnb5n7kgySk9yRAsQDYmyCxzfSky9t": 5,
                },
                "feeMultiplierPermille": 1500,
                "protocolVersion": 9,
            })
        );
        let recovered = FinalizedEpochInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // In non-HR mode, `block_proposers` keeps `Value::Identifier` keys (no
        // base58 stringification). Heights/Credits are u64; core heights u32;
        // protocol_version u32.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "firstBlockTime": 1_700_000_000_000_u64,
                "firstBlockHeight": 100u64,
                "totalBlocksInEpoch": 250u64,
                "firstCoreBlockHeight": 50u32,
                "nextEpochStartCoreBlockHeight": 75u32,
                "totalProcessingFees": 1_000_000u64,
                "totalDistributedStorageFees": 200_000u64,
                "totalCreatedStorageFees": 250_000u64,
                "coreBlockRewards": 500_000u64,
                "blockProposers": {
                    Identifier::new([0xab; 32]): 5u64,
                },
                "feeMultiplierPermille": 1500u64,
                "protocolVersion": 9u32,
            })
        );
        let recovered = FinalizedEpochInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
