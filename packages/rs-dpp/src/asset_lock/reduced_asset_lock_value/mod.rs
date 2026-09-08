use crate::asset_lock::reduced_asset_lock_value::v0::AssetLockValueV0;
use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Bytes32;
use platform_version::version::PlatformVersion;

mod v0;

pub use v0::{AssetLockValueGettersV0, AssetLockValueSettersV0};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    From,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    DecodeUntrusted,
)]
// Stored asset-lock values are decoded from GroveDB proof elements on the
// client before the quorum signature is checked, so the byte budget must be
// enforced by the decoder itself. A valid value is well under 1 KiB (P2PKH
// script, at most `max_asset_lock_usage_attempts` 32-byte tags); the limit
// leaves room for Core's 10,000-byte script ceiling.
#[platform_serialize(limit = 15000, unversioned)]
#[serde(tag = "$formatVersion")]
pub enum AssetLockValue {
    #[serde(rename = "0")]
    V0(AssetLockValueV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for AssetLockValue {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for AssetLockValue {}

impl AssetLockValue {
    pub fn new(
        initial_credit_value: Credits,
        tx_out_script: Vec<u8>,
        remaining_credit_value: Credits,
        used_tags: Vec<Bytes32>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .asset_lock_versions
            .reduced_asset_lock_value
            .default_current_version
        {
            0 => Ok(AssetLockValue::V0(AssetLockValueV0 {
                initial_credit_value,
                tx_out_script,
                remaining_credit_value,
                used_tags,
            })),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ReducedAssetLockValue::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl AssetLockValueGettersV0 for AssetLockValue {
    fn initial_credit_value(&self) -> Credits {
        match self {
            AssetLockValue::V0(v0) => v0.initial_credit_value,
        }
    }

    fn tx_out_script(&self) -> &Vec<u8> {
        match self {
            AssetLockValue::V0(v0) => &v0.tx_out_script,
        }
    }

    fn tx_out_script_owned(self) -> Vec<u8> {
        match self {
            AssetLockValue::V0(v0) => v0.tx_out_script,
        }
    }

    fn remaining_credit_value(&self) -> Credits {
        match self {
            AssetLockValue::V0(v0) => v0.remaining_credit_value,
        }
    }

    fn used_tags_ref(&self) -> &Vec<Bytes32> {
        match self {
            AssetLockValue::V0(v0) => &v0.used_tags,
        }
    }
}

impl AssetLockValueSettersV0 for AssetLockValue {
    fn set_initial_credit_value(&mut self, value: Credits) {
        match self {
            AssetLockValue::V0(v0) => v0.initial_credit_value = value,
        }
    }

    fn set_tx_out_script(&mut self, value: Vec<u8>) {
        match self {
            AssetLockValue::V0(v0) => v0.tx_out_script = value,
        }
    }

    fn set_remaining_credit_value(&mut self, value: Credits) {
        match self {
            AssetLockValue::V0(v0) => v0.remaining_credit_value = value,
        }
    }

    fn set_used_tags(&mut self, tags: Vec<Bytes32>) {
        match self {
            AssetLockValue::V0(v0) => v0.used_tags = tags,
        }
    }

    fn add_used_tag(&mut self, tag: Bytes32) {
        match self {
            AssetLockValue::V0(v0) => v0.used_tags.push(tag),
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use platform_version::version::PlatformVersion;
    use serde_json::json;

    fn fixture() -> AssetLockValue {
        AssetLockValue::new(
            1_000_000,
            vec![0xaa, 0xbb, 0xcc, 0xdd],
            500_000,
            vec![Bytes32::new([0x42; 32])],
            PlatformVersion::latest(),
        )
        .expect("fixture")
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `AssetLockValue` uses the standard `tag = "$formatVersion"`
        // convention. `Bytes32` is base64 in JSON HR, and `tx_out_script`
        // (`Vec<u8>`) is base64 too: `#[json_safe_fields]` annotates it with
        // `serde_bytes_var` (raw bytes in binary, base64 string in JSON).
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "initial_credit_value": 1_000_000,
                "tx_out_script": "qrvM3Q==",
                "remaining_credit_value": 500_000,
                "used_tags": ["QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI="],
            })
        );
        let recovered = AssetLockValue::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::Value;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `#[json_safe_fields]` annotates `tx_out_script` (`Vec<u8>`) with
        // `serde_bytes_var`, so it encodes as `Value::Bytes` (raw bytes, not an
        // array of `U8`). `used_tags` is `Array(Vec<Value::Bytes32>)`.
        // `initial_credit_value` / `remaining_credit_value` are `Credits` (u64);
        // in non-human-readable `Value` they stay `Value::U64`.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "initial_credit_value": 1_000_000u64,
                "tx_out_script": Value::Bytes(vec![0xaa, 0xbb, 0xcc, 0xdd]),
                "remaining_credit_value": 500_000u64,
                "used_tags": [Value::Bytes32([0x42; 32])],
            })
        );
        let recovered = AssetLockValue::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_large_credits_serialize_as_strings_for_js_safety() {
        use crate::serialization::JsonConvertible;
        // `initial_credit_value` exceeds JS `Number.MAX_SAFE_INTEGER` (2^53 - 1),
        // so `#[json_safe_fields]` must emit it as a string in human-readable JSON
        // to avoid silent precision loss when the value crosses into JavaScript.
        // Without the attribute this serializes as a bare number and the
        // assertion below fails.
        let original = AssetLockValue::new(
            9_007_199_254_740_993, // 2^53 + 1, above MAX_SAFE_INTEGER
            vec![0xaa, 0xbb, 0xcc, 0xdd],
            500_000,
            vec![Bytes32::new([0x42; 32])],
            PlatformVersion::latest(),
        )
        .expect("fixture");
        let json = original.to_json().expect("to_json");
        assert_eq!(json["initial_credit_value"], json!("9007199254740993"));
        // Values within the safe range stay numbers.
        assert_eq!(json["remaining_credit_value"], json!(500_000));
        // And the string form round-trips back to the exact u64.
        let recovered = AssetLockValue::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }
}

#[cfg(test)]
mod deserialize_limit_tests {
    use super::*;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};

    /// Bincode-encode the V0 shape by hand so the `tx_out_script` length prefix
    /// can claim more bytes than exist in the payload.
    fn payload_with_script_length(fake_len: u64) -> Vec<u8> {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let mut buf = Vec::new();
        // enum discriminant: V0
        buf.extend_from_slice(&bincode::encode_to_vec(0u32, config).unwrap());
        // initial_credit_value
        buf.extend_from_slice(&bincode::encode_to_vec(1_000u64, config).unwrap());
        // tx_out_script length prefix, with no bytes following it
        buf.extend_from_slice(&bincode::encode_to_vec(fake_len, config).unwrap());
        buf
    }

    /// A proof element is untrusted input: a length prefix must be rejected
    /// against the byte budget before it sizes an allocation. Without the
    /// limit this was `vec.resize(8_000_000_000, 0)` and an abort.
    #[test]
    fn rejects_script_length_prefix_beyond_budget_without_allocating() {
        let payload = payload_with_script_length(8_000_000_000);
        let err = AssetLockValue::deserialize_from_bytes(&payload)
            .expect_err("oversized length prefix must be rejected");
        assert!(
            matches!(err, ProtocolError::MaxEncodedBytesReachedError { .. }),
            "unexpected error: {err}"
        );
    }

    /// The largest value the server can legitimately store must stay inside
    /// the budget on both the encode and decode side, or the node could fail
    /// to persist it.
    #[test]
    fn largest_valid_value_round_trips_under_limit() {
        let platform_version = PlatformVersion::latest();
        let max_tags = platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .max_asset_lock_usage_attempts as usize;
        let original = AssetLockValue::new(
            u64::MAX,
            vec![0xffu8; 10_000],
            u64::MAX,
            vec![Bytes32::new([0xff; 32]); max_tags],
            platform_version,
        )
        .expect("value");
        let bytes = original.serialize_to_bytes().expect("serialize");
        let recovered = AssetLockValue::deserialize_from_bytes(&bytes).expect("deserialize");
        assert_eq!(original, recovered);
    }
}
