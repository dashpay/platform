#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use ::serde::{Deserialize, Serialize};
use platform_value::Value;
use std::convert::TryFrom;

use crate::identifier::Identifier;
use crate::util::hash::hash_double;
use dashcore::OutPoint;

/// Instant Asset Lock Proof is a part of Identity Create and Identity Topup
/// transitions. It is a proof that specific output of dash is locked in credits
/// pull and the transitions can mint credits and populate identity's balance.
/// To prove that the output is locked, a height where transaction was chain locked is provided.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainAssetLockProof {
    /// Core height on which the asset lock transaction was chain locked or higher
    pub core_chain_locked_height: u32,
    /// A reference to Asset Lock Special Transaction ID and output index in the payload
    pub out_point: OutPoint,
}

impl TryFrom<Value> for ChainAssetLockProof {
    type Error = platform_value::Error;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value)
    }
}

impl ChainAssetLockProof {
    pub fn new(core_chain_locked_height: u32, out_point: [u8; 36]) -> Self {
        Self {
            core_chain_locked_height,
            out_point: OutPoint::from(out_point),
        }
    }

    /// Create identifier
    pub fn create_identifier(&self) -> Identifier {
        let outpoint_bytes: [u8; 36] = self.out_point.into();

        let hash = hash_double(outpoint_bytes.as_slice());

        Identifier::new(hash)
    }
}

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;
    use dashcore::{OutPoint, Txid};
    use std::str::FromStr;

    #[test]
    fn chain_asset_lock_proof_json_round_trip() {
        let txid_hex = "e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d";
        let txid = Txid::from_str(txid_hex).unwrap();
        let proof = ChainAssetLockProof {
            core_chain_locked_height: 11,
            out_point: OutPoint { txid, vout: 1 },
        };

        let json = proof.to_json().expect("to_json should succeed");

        // OutPoint should be "txid:vout" string (human-readable serde_json)
        assert!(
            json["outPoint"].is_string(),
            "outPoint should be a string, got: {:?}",
            json["outPoint"]
        );
        assert!(
            json["outPoint"].as_str().unwrap().contains(":"),
            "outPoint should contain ':'"
        );
        assert_eq!(json["coreChainLockedHeight"].as_u64().unwrap(), 11);

        let restored = ChainAssetLockProof::from_json(json).expect("from_json should succeed");
        assert_eq!(proof, restored);
    }

    #[test]
    fn chain_asset_lock_proof_value_round_trip() {
        use crate::serialization::ValueConvertible;
        let txid_hex = "e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d";
        let txid = Txid::from_str(txid_hex).unwrap();
        let proof = ChainAssetLockProof {
            core_chain_locked_height: 11,
            out_point: OutPoint { txid, vout: 1 },
        };

        let obj = proof.to_object().expect("to_object should succeed");
        let restored = ChainAssetLockProof::from_object(obj).expect("from_object should succeed");
        assert_eq!(proof, restored);
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
    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Txid};
    use platform_value::platform_value;
    use serde_json::json;
    use std::str::FromStr;

    fn fixture() -> ChainAssetLockProof {
        ChainAssetLockProof {
            core_chain_locked_height: 12345,
            out_point: OutPoint::from_str(
                "0000000000000000000000000000000000000000000000000000000000000001:0",
            )
            .expect("outpoint"),
        }
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `ChainAssetLockProof` is a plain struct with `rename_all = "camelCase"`.
        // `out_point` uses dashcore's `OutPoint` serde directly (the local
        // `outpoint_serde` wrapper was dropped once dashcore #708 landed the
        // unified visitor upstream), which is HR-aware: in JSON this emits the
        // `"<txid>:<vout>"` string form. `core_chain_locked_height` is `u32`;
        // JSON erases the size — see the value-path assertion.
        // The HR string form mirrors the input we passed to `from_str`.
        assert_eq!(
            json,
            json!({
                "coreChainLockedHeight": 12345,
                "outPoint": "0000000000000000000000000000000000000000000000000000000000000001:0",
            })
        );
        let recovered = ChainAssetLockProof::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `12345u32` locks `Value::U32`. The non-HR path of `OutPoint::serialize`
        // emits a `{txid, vout}` STRUCT, NOT the `"<txid>:<vout>"` string —
        // and `Txid` itself serializes as a 32-byte array on the non-HR path
        // (collapsing to `Value::Bytes32` via platform_value's sized-bytes
        // detection). `vout` is `u32`. This is exactly the dual-shape behaviour
        // dashcore's unified-visitor `OutPoint` serde (upstream #708; it
        // replaced the local `outpoint_serde` wrapper) round-trips via
        // `ContentDeserializer` (the HR=true / non-HR=false split).
        // NOTE on byte order: the JSON/hex form (`00...01`, lowest nibble at
        // the end) is REVERSED from the raw-bytes form. dashcore's `Txid`
        // follows the Bitcoin convention — `as_byte_array()` returns the raw
        // buffer where index 0 holds what shows as the LAST hex digit. So
        // the displayed `00...01` corresponds to raw `[0x01, 0, 0, ..., 0]`.
        // dashcore's `OutPoint` serde bridges the two shapes.
        let mut raw = [0u8; 32];
        raw[0] = 0x01;
        assert_eq!(
            value,
            platform_value!({
                "coreChainLockedHeight": 12345u32,
                "outPoint": {
                    "txid": platform_value::Value::Bytes32(raw),
                    "vout": 0u32,
                },
            })
        );
        let recovered = ChainAssetLockProof::from_object(value).expect("from_object");
        assert_eq!(original, recovered);

        // Sanity check that the byte-array matches the real Txid bytes (so
        // any future flip in dashcore's byte-order convention fails loud).
        let txid_from_str =
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap();
        assert_eq!(txid_from_str.as_byte_array(), &raw);
    }
}
