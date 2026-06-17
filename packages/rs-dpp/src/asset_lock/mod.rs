use crate::asset_lock::reduced_asset_lock_value::AssetLockValue;

pub mod reduced_asset_lock_value;

pub type PastAssetLockStateTransitionHashes = Vec<Vec<u8>>;

/// An enumeration of the possible states when querying platform to get the stored state of an outpoint
/// representing if the asset lock was already used or not.
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "$type")]
pub enum StoredAssetLockInfo {
    /// The asset lock was fully consumed in the past
    FullyConsumed,
    /// The asset lock was partially consumed, and we stored the asset lock value in the state
    PartiallyConsumed(AssetLockValue),
    /// The asset lock is not yet known to Platform
    NotPresent,
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for StoredAssetLockInfo {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for StoredAssetLockInfo {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::{platform_value, Bytes32, Value};
    use platform_version::version::PlatformVersion;
    use serde_json::json;

    fn partially_consumed_fixture() -> StoredAssetLockInfo {
        let asset_lock_value = AssetLockValue::new(
            1_000_000,
            vec![0xaa, 0xbb, 0xcc, 0xdd],
            500_000,
            vec![Bytes32::new([0x42; 32])],
            PlatformVersion::latest(),
        )
        .expect("fixture");
        StoredAssetLockInfo::PartiallyConsumed(asset_lock_value)
    }

    // `StoredAssetLockInfo` is internally tagged (`#[serde(tag = "$type")]`):
    // unit variants serialize as `{"$type": "FullyConsumed"}`, and the
    // `PartiallyConsumed` newtype variant flattens its inner `AssetLockValue`
    // (itself `tag = "$formatVersion"`) up to the same wire level, giving
    // `{"$type": "PartiallyConsumed", "$formatVersion": "0", ...}`. The `$`
    // prefix follows the codebase rule: the discriminator is `$`-prefixed when
    // other `$`-fields (here `$formatVersion`) share the same wire level.

    #[test]
    fn json_round_trip_fully_consumed() {
        use crate::serialization::JsonConvertible;
        let original = StoredAssetLockInfo::FullyConsumed;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!({ "$type": "FullyConsumed" }));
        let recovered = StoredAssetLockInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_not_present() {
        use crate::serialization::JsonConvertible;
        let original = StoredAssetLockInfo::NotPresent;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!({ "$type": "NotPresent" }));
        let recovered = StoredAssetLockInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_partially_consumed() {
        use crate::serialization::JsonConvertible;
        let original = partially_consumed_fixture();
        let json = original.to_json().expect("to_json");
        // Internal `$type` tag sits flat alongside the inner `AssetLockValue`'s
        // own `$formatVersion` tag (`tag = "$formatVersion"`). `Bytes32` is
        // base64 in JSON HR, and `tx_out_script` (`Vec<u8>`) is base64 too via
        // `#[json_safe_fields]`'s `serde_bytes_var` annotation.
        assert_eq!(
            json,
            json!({
                "$type": "PartiallyConsumed",
                "$formatVersion": "0",
                "initial_credit_value": 1_000_000,
                "tx_out_script": "qrvM3Q==",
                "remaining_credit_value": 500_000,
                "used_tags": ["QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI="],
            })
        );
        let recovered = StoredAssetLockInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_fully_consumed() {
        use crate::serialization::ValueConvertible;
        let original = StoredAssetLockInfo::FullyConsumed;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!({ "$type": "FullyConsumed" }));
        let recovered = StoredAssetLockInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_not_present() {
        use crate::serialization::ValueConvertible;
        let original = StoredAssetLockInfo::NotPresent;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!({ "$type": "NotPresent" }));
        let recovered = StoredAssetLockInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_partially_consumed() {
        use crate::serialization::ValueConvertible;
        let original = partially_consumed_fixture();
        let value = original.to_object().expect("to_object");
        // `Credits` (u64) → `Value::U64`. `tx_out_script` (`Vec<u8>`) →
        // `Value::Bytes` via `#[json_safe_fields]`. `used_tags` →
        // `Array(Vec<Value::Bytes32>)`.
        assert_eq!(
            value,
            platform_value!({
                "$type": "PartiallyConsumed",
                "$formatVersion": "0",
                "initial_credit_value": 1_000_000u64,
                "tx_out_script": Value::Bytes(vec![0xaa, 0xbb, 0xcc, 0xdd]),
                "remaining_credit_value": 500_000u64,
                "used_tags": [Value::Bytes32([0x42; 32])],
            })
        );
        let recovered = StoredAssetLockInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
