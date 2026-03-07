use crate::serialization::{json_safe_fields, ValueConvertible};
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
use crate::block::epoch::{Epoch, EPOCH_0};
use crate::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const DEFAULT_BLOCK_INFO: BlockInfo = BlockInfo {
    time_ms: 0,
    height: 0,
    core_height: 0,
    epoch: EPOCH_0,
};

// We make this immutable because it should never be changed or updated
// Extended block info however is not immutable
// @immutable
/// Block information
#[json_safe_fields]
#[cfg_attr(
    feature = "json-conversion",
    derive(JsonConvertible)
)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Encode, Decode, Serialize, Deserialize, ValueConvertible)]
#[serde(rename_all = "camelCase")]
pub struct BlockInfo {
    /// Block time in milliseconds
    pub time_ms: TimestampMillis,

    /// Block height
    pub height: BlockHeight,

    /// Core height
    pub core_height: CoreBlockHeight,

    /// Current fee epoch
    pub epoch: Epoch,
}

impl fmt::Display for BlockInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BlockInfo {{ time_ms: {}, height: {}, core_height: {}, epoch: {} }}",
            self.time_ms, self.height, self.core_height, self.epoch.index
        )
    }
}

// Implementing PartialOrd for BlockInfo based on height
impl PartialOrd for BlockInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Implementing Ord for BlockInfo based on height
impl Ord for BlockInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height)
    }
}

impl BlockInfo {
    // TODO: It's not actually a genesis one. We should use just default to avoid confusion
    /// Create block info for genesis block
    pub fn genesis() -> BlockInfo {
        BlockInfo::default()
    }

    /// Create default block with specified time
    pub fn default_with_time(time_ms: TimestampMillis) -> BlockInfo {
        BlockInfo {
            time_ms,
            ..Default::default()
        }
    }

    /// Create default block with specified height
    pub fn default_with_height(height: BlockHeight) -> BlockInfo {
        BlockInfo {
            height,
            ..Default::default()
        }
    }

    /// Create default block with specified height and time
    pub fn default_with_height_and_time(
        height: BlockHeight,
        time_ms: TimestampMillis,
    ) -> BlockInfo {
        BlockInfo {
            height,
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


#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::block::epoch::Epoch;
    use crate::serialization::JsonConvertible;

    #[test]
    fn block_info_json_round_trip() {
        let block_info = BlockInfo {
            time_ms: 1_700_000_000_000u64,
            height: 12345678u64,
            core_height: 900_000u32,
            epoch: Epoch::new(42).unwrap(),
        };

        let json = block_info.to_json().expect("to_json should succeed");
        assert!(json["timeMs"].is_number());
        assert_eq!(json["timeMs"].as_u64().unwrap(), 1700000000000);
        assert!(json["height"].is_number());
        assert_eq!(json["height"].as_u64().unwrap(), 12345678);
        assert!(json["coreHeight"].is_number());
        assert_eq!(json["coreHeight"].as_u64().unwrap(), 900_000);

        let restored = BlockInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(block_info, restored);
    }

    #[test]
    fn block_info_value_round_trip() {
        let block_info = BlockInfo {
            time_ms: u64::MAX,
            height: 999u64,
            core_height: 100u32,
            epoch: Epoch::new(0).unwrap(),
        };

        let obj = block_info.to_object().expect("to_object should succeed");
        let time_val = obj
            .get("timeMs")
            .expect("get should not fail on map")
            .expect("timeMs key must exist");
        assert!(
            time_val.is_integer(),
            "Value timeMs should be an integer type, got: {:?}",
            time_val
        );

        let restored = BlockInfo::from_object(obj).expect("from_object should succeed");
        assert_eq!(block_info, restored);
    }

    #[test]
    fn block_info_max_u64_json_round_trip() {
        let block_info = BlockInfo {
            time_ms: u64::MAX,
            height: u64::MAX,
            core_height: u32::MAX,
            epoch: Epoch::new(100).unwrap(),
        };

        let json = block_info.to_json().expect("to_json should succeed");
        // u64::MAX > JS MAX_SAFE_INTEGER, serialized as string
        assert!(json["timeMs"].is_string());
        assert_eq!(json["timeMs"].as_str().unwrap(), u64::MAX.to_string());
        assert!(json["height"].is_string());
        assert_eq!(json["height"].as_str().unwrap(), u64::MAX.to_string());

        let restored = BlockInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(block_info, restored);
    }
}
