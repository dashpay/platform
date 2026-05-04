#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use ::serde::{Deserialize, Serialize};
use platform_value::Value;
use std::convert::TryFrom;

use crate::util::hash::hash_double;
use crate::{identifier::Identifier, ProtocolError};
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
    // TODO(dashcore-PR-link-pending): remove `serde(with = "outpoint_serde")` once the
    // upstream fix to `dashcore::serde_struct_human_string_impl!` (unified visitor for
    // string + struct shapes) lands and we bump the dashcore dependency. The local
    // wrapper exists only because dashcore's OutPoint::deserialize uses two
    // is_human_readable-disjoint visitors, which fails through serde's
    // ContentDeserializer (always reports HR=true) — see Critical-3 / B1 in
    // docs/json-value-unification-plan.md and §10b.
    #[serde(with = "outpoint_serde")]
    pub out_point: OutPoint,
}

/// Local Deserialize wrapper for [`OutPoint`] that accepts both shapes — the
/// `"txid:vout"` string form (human-readable serde_json) AND the
/// `{txid, vout}` struct form (non-human-readable bincode / platform_value) —
/// regardless of the deserializer's `is_human_readable` flag.
///
/// Required because dashcore's built-in `OutPoint::deserialize` uses two
/// completely disjoint visitors (one per HR branch). Through serde's
/// `ContentDeserializer` (used for any internally-tagged enum like
/// `AssetLockProof`'s `#[serde(tag = "type")]`), `is_human_readable` falsely
/// reports `true` even when the buffered value is the non-HR struct form,
/// which causes the HR `StringVisitor` to be invoked on a `Content::Map`,
/// failing with `"invalid type: map, expected an OutPoint"`.
///
/// Mirrors the dual-shape visitor pattern in
/// `rs-platform-value::types::{bytes_32, binary_data, identifier}` and in
/// `rs-dpp::serialization::serde_bytes`.
mod outpoint_serde {
    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Txid};
    use serde::de::{self, Deserialize, MapAccess, SeqAccess, Visitor};
    use serde::{Deserializer, Serialize, Serializer};
    use std::fmt;
    use std::str::FromStr;

    pub fn serialize<S: Serializer>(p: &OutPoint, serializer: S) -> Result<S::Ok, S::Error> {
        // Delegate to dashcore's own Serialize — it already does the right thing
        // (HR: "txid:vout" string, non-HR: {txid, vout} struct).
        p.serialize(serializer)
    }

    /// Wraps `Txid` with a Deserialize that accepts BOTH a 64-char hex string
    /// AND a 32-byte array, regardless of `is_human_readable`. Same
    /// `ContentDeserializer` quirk as `OutPoint` itself; the upstream dashcore
    /// `hash_newtype!` macro inherits the disjoint-visitor bug.
    struct TxidCompat(Txid);

    impl<'de> Deserialize<'de> for TxidCompat {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct TxidVisitor;

            impl<'de> Visitor<'de> for TxidVisitor {
                type Value = Txid;

                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("Txid as 64-char hex string or 32-byte array")
                }

                fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                    Txid::from_str(v).map_err(E::custom)
                }

                fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                    if v.len() != 32 {
                        return Err(E::invalid_length(v.len(), &self));
                    }
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(v);
                    Ok(Txid::from_byte_array(arr))
                }

                fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                    self.visit_bytes(&v)
                }

                fn visit_seq<A: SeqAccess<'de>>(
                    self,
                    mut seq: A,
                ) -> Result<Self::Value, A::Error> {
                    let mut arr = [0u8; 32];
                    for (i, slot) in arr.iter_mut().enumerate() {
                        *slot = seq
                            .next_element::<u8>()?
                            .ok_or_else(|| <A::Error as de::Error>::invalid_length(i, &self))?;
                    }
                    Ok(Txid::from_byte_array(arr))
                }
            }

            // Same `is_human_readable` branching strategy as
            // `crate::serialization::serde_bytes` — bincode (the binary path
            // used by `PlatformSerialize`/`PlatformDeserialize`) doesn't
            // support `deserialize_any`, so the non-HR branch picks an
            // explicit shape hint.
            if deserializer.is_human_readable() {
                deserializer.deserialize_any(TxidVisitor).map(TxidCompat)
            } else {
                deserializer.deserialize_byte_buf(TxidVisitor).map(TxidCompat)
            }
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<OutPoint, D::Error> {
        struct OutPointVisitor;

        impl<'de> Visitor<'de> for OutPointVisitor {
            type Value = OutPoint;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an OutPoint as either \"txid:vout\" string or {txid, vout} struct")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                OutPoint::from_str(v).map_err(E::custom)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut txid: Option<Txid> = None;
                let mut vout: Option<u32> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "txid" => txid = Some(map.next_value::<TxidCompat>()?.0),
                        "vout" => vout = Some(map.next_value()?),
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                Ok(OutPoint {
                    txid: txid.ok_or_else(|| <A::Error as de::Error>::missing_field("txid"))?,
                    vout: vout.ok_or_else(|| <A::Error as de::Error>::missing_field("vout"))?,
                })
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let txid = seq
                    .next_element::<TxidCompat>()?
                    .ok_or_else(|| <A::Error as de::Error>::invalid_length(0, &self))?
                    .0;
                let vout: u32 = seq
                    .next_element()?
                    .ok_or_else(|| <A::Error as de::Error>::invalid_length(1, &self))?;
                Ok(OutPoint { txid, vout })
            }
        }

        if deserializer.is_human_readable() {
            // Covers true HR (serde_json sees a string) AND
            // ContentDeserializer (HR=true even when wrapping a struct from a
            // non-HR source like platform_value).
            deserializer.deserialize_any(OutPointVisitor)
        } else {
            // Non-HR (bincode): the wire shape is `{txid, vout}` struct.
            deserializer.deserialize_struct(
                "OutPoint",
                &["txid", "vout"],
                OutPointVisitor,
            )
        }
    }
}

impl TryFrom<Value> for ChainAssetLockProof {
    type Error = platform_value::Error;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value)
    }
}

impl ChainAssetLockProof {
    pub fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }
    pub fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        self.to_object()
    }

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

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
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
        // The local `outpoint_serde` wrapper (commit 09c0a2b771) delegates to
        // dashcore's `OutPoint::serialize`, which is HR-aware: in JSON this
        // emits the `"<txid>:<vout>"` string form. `core_chain_locked_height`
        // is `u32`; JSON erases the size — see the value-path assertion.
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
        // the local `outpoint_serde` wrapper has to round-trip via
        // `ContentDeserializer` (the HR=true / non-HR=false split that broke
        // round-tripping before commit 09c0a2b771).
        // NOTE on byte order: the JSON/hex form (`00...01`, lowest nibble at
        // the end) is REVERSED from the raw-bytes form. dashcore's `Txid`
        // follows the Bitcoin convention — `as_byte_array()` returns the raw
        // buffer where index 0 holds what shows as the LAST hex digit. So
        // the displayed `00...01` corresponds to raw `[0x01, 0, 0, ..., 0]`.
        // The local `outpoint_serde` wrapper bridges the two shapes.
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
        let txid_from_str = Txid::from_str(
            "0000000000000000000000000000000000000000000000000000000000000001",
        )
        .unwrap();
        assert_eq!(txid_from_str.as_byte_array(), &raw);
    }
}
