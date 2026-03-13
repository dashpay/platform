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
