use std::convert::{TryFrom, TryInto};

use dashcore::{OutPoint, Transaction};

use serde::{Deserialize, Deserializer, Serialize};

use bincode::{Decode, Encode};

pub use instant::*;
use platform_value::Value;
#[cfg(feature = "validation")]
use platform_version::version::PlatformVersion;
use serde::de::Error;

use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::prelude::Identifier;
#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
use crate::{ProtocolError, SerdeParsingError};

pub mod chain;
pub mod instant;
pub mod validate_asset_lock_transaction_structure;

// TODO: Serialization with bincode
// TODO: Consider use Box for InstantAssetLockProof
//
// Wire-shape note: this is an *internally-tagged* enum (`#[serde(tag = "$type")]`
// with no `content`). serde's internal tagging works on newtype variants whose
// inner is a struct — both `InstantAssetLockProof` and `ChainAssetLockProof`
// qualify — so the inner struct's fields are flattened next to the `type`
// discriminator: `{"$type": "instant", "instantLock": ..., "transaction": ...,
// "outputIndex": ...}`. This matches the convention applied to other tagged
// unions exposed to JS (see `AddressWitness`, `AddressFundsFeeStrategyStep`).
// Bincode `Encode`/`Decode` derives are independent of serde, so consensus
// binary format is unaffected.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Encode, Decode)]
#[serde(tag = "$type", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum AssetLockProof {
    Instant(#[bincode(with_serde)] InstantAssetLockProof),
    Chain(#[bincode(with_serde)] ChainAssetLockProof),
}

/// Wire-shape Deserialize uses the same internal-tag layout the Serialize derive
/// produces, but routes the instant variant through `RawInstantLockProof` so the
/// dashcore `InstantLock` can be reconstructed from its raw bytes form.
#[derive(Deserialize)]
#[serde(tag = "$type", rename_all = "camelCase")]
enum RawAssetLockProof {
    Instant(RawInstantLockProof),
    Chain(ChainAssetLockProof),
}

impl TryFrom<RawAssetLockProof> for AssetLockProof {
    type Error = ProtocolError;

    fn try_from(value: RawAssetLockProof) -> Result<Self, Self::Error> {
        match value {
            RawAssetLockProof::Instant(raw_instant_lock) => {
                let instant_lock = raw_instant_lock.try_into()?;

                Ok(AssetLockProof::Instant(instant_lock))
            }
            RawAssetLockProof::Chain(chain) => Ok(AssetLockProof::Chain(chain)),
        }
    }
}

impl<'de> Deserialize<'de> for AssetLockProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawAssetLockProof::deserialize(deserializer)?;
        raw.try_into().map_err(|e: ProtocolError| {
            D::Error::custom(format!(
                "expected to be able to deserialize asset lock proof: {}",
                e
            ))
        })
    }
}

impl Default for AssetLockProof {
    fn default() -> Self {
        Self::Instant(InstantAssetLockProof::default())
    }
}

#[cfg(feature = "json-conversion")]
impl crate::serialization::JsonConvertible for AssetLockProof {}

#[cfg(feature = "value-conversion")]
impl crate::serialization::ValueConvertible for AssetLockProof {}

impl AsRef<AssetLockProof> for AssetLockProof {
    fn as_ref(&self) -> &AssetLockProof {
        self
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
    use dashcore::OutPoint;
    use platform_value::platform_value;
    use serde_json::json;
    use std::str::FromStr;

    /// Non-default variant (`Chain` with non-zero core height + a real
    /// outpoint) so the wire-shape assertion catches silent variant flip /
    /// inner-zero on round-trip — the previous fixture used `Default::default`
    /// (`Instant` zero proof).
    fn fixture() -> AssetLockProof {
        let out_point = OutPoint::from_str(
            "0000000000000000000000000000000000000000000000000000000000000001:1",
        )
        .expect("outpoint");
        AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 12_345,
            out_point,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `AssetLockProof` is internally tagged (`#[serde(tag = "$type")]`), so
        // the inner `ChainAssetLockProof`'s fields are flattened next to the
        // discriminator. Surprising shape: `OutPoint` has a *string-form*
        // Serialize impl ("<txid>:<vout>") in dashcore which JSON consumes
        // as-is — so on the JSON wire, `outPoint` is a single string. The
        // platform_value layer goes through a different path (see the
        // value-side test below) and produces a typed Map with `Bytes32` txid
        // and `U32` vout. `coreChainLockedHeight` is `u32`; JSON erases the
        // size — see the value-path assertion.
        assert_eq!(
            json,
            json!({
                "$type": "chain",
                "coreChainLockedHeight": 12_345,
                "outPoint": "0000000000000000000000000000000000000000000000000000000000000001:1",
            })
        );
        let recovered = AssetLockProof::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // platform_value path: `OutPoint` serializes via its derived structural
        // impl producing a Map { txid: Bytes32, vout: U32 } (NOT the string form
        // produced on the JSON side). `coreChainLockedHeight` is `u32` so
        // `12_345u32` locks in `Value::U32`.
        let mut txid_bytes = [0u8; 32];
        txid_bytes[0] = 1;
        assert_eq!(
            value,
            platform_value!({
                "$type": "chain",
                "coreChainLockedHeight": 12_345u32,
                "outPoint": {
                    "txid": platform_value::Value::Bytes32(txid_bytes),
                    "vout": 1u32,
                },
            })
        );
        let recovered = AssetLockProof::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
pub enum AssetLockProofType {
    Instant = 0,
    Chain = 1,
}

impl TryFrom<u8> for AssetLockProofType {
    type Error = SerdeParsingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Instant),
            1 => Ok(Self::Chain),
            _ => Err(SerdeParsingError::new("Unexpected asset lock proof type")),
        }
    }
}

impl TryFrom<u64> for AssetLockProofType {
    type Error = SerdeParsingError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Instant),
            1 => Ok(Self::Chain),
            _ => Err(SerdeParsingError::new("Unexpected asset lock proof type")),
        }
    }
}

// TODO: Versioning
impl AssetLockProof {
    pub fn type_from_raw_value(value: &Value) -> Option<AssetLockProofType> {
        let proof_type_res = value.get_integer::<u8>("type");

        match proof_type_res {
            Ok(proof_type_int) => {
                let proof_type = AssetLockProofType::try_from(proof_type_int);
                proof_type.ok()
            }
            Err(_) => None,
        }
    }

    pub fn create_identifier(&self) -> Result<Identifier, ProtocolError> {
        match self {
            AssetLockProof::Instant(instant_proof) => instant_proof.create_identifier(),
            AssetLockProof::Chain(chain_proof) => Ok(chain_proof.create_identifier()),
        }
    }

    pub fn output_index(&self) -> u32 {
        match self {
            AssetLockProof::Instant(proof) => proof.output_index(),
            AssetLockProof::Chain(proof) => proof.out_point.vout,
        }
    }

    pub fn out_point(&self) -> Option<OutPoint> {
        match self {
            AssetLockProof::Instant(proof) => proof.out_point(),
            AssetLockProof::Chain(proof) => Some(proof.out_point),
        }
    }

    pub fn transaction(&self) -> Option<&Transaction> {
        match self {
            AssetLockProof::Instant(is_lock) => Some(is_lock.transaction()),
            AssetLockProof::Chain(_chain_lock) => None,
        }
    }

    /// Validate the structure of the asset lock proof
    #[cfg(feature = "validation")]
    pub fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match self {
            AssetLockProof::Instant(proof) => proof.validate_structure(platform_version),
            AssetLockProof::Chain(_) => Ok(SimpleConsensusValidationResult::default()),
        }
    }
}

// Canonical `TryFrom<Value> for AssetLockProof` is provided via the
// `Deserialize` impl above (which routes through `RawAssetLockProof` for
// the instant-lock raw-bytes shape) and `platform_value::from_value`. The
// previous hack here accepted legacy integer-tagged
// (`{type: 0|1, ...fields}`) and externally-tagged
// (`{Instant: {...}}`) shapes — both predated the
// `#[serde(tag = "$type")]` Critical-2 fix. Audit (Phase D step 6)
// confirmed all currently-flowing values are canonical-tagged
// (string `type`), so the hacks were dead.

impl TryFrom<&Value> for AssetLockProof {
    type Error = ProtocolError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value.clone()).map_err(ProtocolError::ValueError)
    }
}

impl TryFrom<Value> for AssetLockProof {
    type Error = ProtocolError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value).map_err(ProtocolError::ValueError)
    }
}

// `TryInto<Value>` impls (and the inherent `to_raw_object` that mirrored
// them) used to live here, producing *untagged* `Value` (drops the variant
// tag entirely). They were structurally asymmetric with the canonical
// Deserialize, which expects the `type: "instant" | "chain"` discriminator
// to route through `RawAssetLockProof`. Confirmed zero production callers,
// so deleted in Phase D step 6. Use canonical `ValueConvertible::to_object`
// — it produces the correctly-tagged shape that `Deserialize` accepts on
// the way back.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use dashcore::{OutPoint, Txid};
    use std::str::FromStr;

    /// JSON wire shape is internally tagged: `{type, ...flattened inner fields}`,
    /// no `data` wrapper. This guards against accidental reintroduction of the
    /// old adjacent-tagged `{type, data: {...}}` shape and against the divergence
    /// from the `AddressWitness` / `AddressFundsFeeStrategyStep` precedent.
    #[test]
    fn chain_variant_serializes_with_internal_tag() {
        let txid =
            Txid::from_str("e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d")
                .unwrap();
        let proof = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 11,
            out_point: OutPoint { txid, vout: 1 },
        });

        let json = serde_json::to_value(&proof).expect("serialize");

        assert_eq!(json["$type"], "chain");
        assert_eq!(json["coreChainLockedHeight"], 11);
        assert!(
            json.get("data").is_none(),
            "should not have a `data` wrapper, got: {}",
            json
        );

        // Round-trip
        let restored: AssetLockProof = serde_json::from_value(json).expect("deserialize");
        assert_eq!(proof, restored);
    }

    mod asset_lock_proof_type_try_from {
        use super::*;

        #[test]
        fn u8_instant_type() {
            let proof_type = AssetLockProofType::try_from(0u8).expect("should parse type 0");
            assert!(matches!(proof_type, AssetLockProofType::Instant));
        }

        #[test]
        fn u8_chain_type() {
            let proof_type = AssetLockProofType::try_from(1u8).expect("should parse type 1");
            assert!(matches!(proof_type, AssetLockProofType::Chain));
        }

        #[test]
        fn u8_invalid_type() {
            let result = AssetLockProofType::try_from(2u8);
            assert!(result.is_err());
        }

        #[test]
        fn u8_max_invalid_type() {
            let result = AssetLockProofType::try_from(255u8);
            assert!(result.is_err());
        }

        #[test]
        fn u64_instant_type() {
            let proof_type = AssetLockProofType::try_from(0u64).expect("should parse type 0");
            assert!(matches!(proof_type, AssetLockProofType::Instant));
        }

        #[test]
        fn u64_chain_type() {
            let proof_type = AssetLockProofType::try_from(1u64).expect("should parse type 1");
            assert!(matches!(proof_type, AssetLockProofType::Chain));
        }

        #[test]
        fn u64_invalid_type() {
            let result = AssetLockProofType::try_from(2u64);
            assert!(result.is_err());
        }

        #[test]
        fn u64_large_invalid_type() {
            let result = AssetLockProofType::try_from(u64::MAX);
            assert!(result.is_err());
        }
    }

    mod chain_asset_lock_proof {
        use super::*;

        fn make_chain_proof() -> ChainAssetLockProof {
            ChainAssetLockProof::new(100, [0xAB; 36])
        }

        #[test]
        fn chain_proof_construction() {
            let proof = ChainAssetLockProof::new(42, [0x01; 36]);
            assert_eq!(proof.core_chain_locked_height, 42);
        }

        #[test]
        fn chain_proof_create_identifier_deterministic() {
            let proof = make_chain_proof();
            let id1 = proof.create_identifier();
            let id2 = proof.create_identifier();
            assert_eq!(id1, id2);
        }

        #[test]
        fn different_outpoints_produce_different_identifiers() {
            let proof_a = ChainAssetLockProof::new(100, [0xAA; 36]);
            let proof_b = ChainAssetLockProof::new(100, [0xBB; 36]);
            assert_ne!(proof_a.create_identifier(), proof_b.create_identifier());
        }

        #[test]
        fn chain_proof_equality() {
            let a = ChainAssetLockProof::new(10, [0x01; 36]);
            let b = ChainAssetLockProof::new(10, [0x01; 36]);
            assert_eq!(a, b);
        }

        #[test]
        fn chain_proof_inequality_height() {
            let a = ChainAssetLockProof::new(10, [0x01; 36]);
            let b = ChainAssetLockProof::new(20, [0x01; 36]);
            assert_ne!(a, b);
        }
    }

    mod asset_lock_proof_methods {
        use super::*;

        fn make_chain_lock_proof() -> AssetLockProof {
            let chain_proof = ChainAssetLockProof::new(50, [0xCC; 36]);
            AssetLockProof::Chain(chain_proof)
        }

        #[test]
        fn default_is_instant() {
            let proof = AssetLockProof::default();
            assert!(matches!(proof, AssetLockProof::Instant(_)));
        }

        #[test]
        fn as_ref_returns_self() {
            let proof = make_chain_lock_proof();
            let reference: &AssetLockProof = proof.as_ref();
            assert_eq!(&proof, reference);
        }

        #[test]
        fn chain_proof_output_index() {
            let mut out_point_bytes = [0u8; 36];
            // Set vout (last 4 bytes in little-endian) to 3
            out_point_bytes[32] = 3;
            let chain_proof = ChainAssetLockProof::new(50, out_point_bytes);
            let proof = AssetLockProof::Chain(chain_proof);
            assert_eq!(proof.output_index(), 3);
        }

        #[test]
        fn chain_proof_out_point_is_some() {
            let proof = make_chain_lock_proof();
            assert!(proof.out_point().is_some());
        }

        #[test]
        fn chain_proof_transaction_is_none() {
            let proof = make_chain_lock_proof();
            assert!(proof.transaction().is_none());
        }

        #[test]
        fn chain_proof_to_object_canonical() {
            // After Phase D step 6, `to_raw_object` (which produced an
            // untagged Value) was deleted. Canonical
            // `ValueConvertible::to_object` produces the correctly-tagged
            // shape that round-trips through `Deserialize`.
            use crate::serialization::ValueConvertible;
            let proof = make_chain_lock_proof();
            let result = proof.to_object();
            assert!(result.is_ok());
        }

        #[test]
        fn chain_proof_create_identifier() {
            let proof = make_chain_lock_proof();
            let id = proof.create_identifier();
            assert!(id.is_ok());
        }
    }

    mod try_from_value {
        use super::*;

        #[test]
        fn chain_proof_value_round_trip() {
            // Canonical `ValueConvertible::to_object` produces a tagged
            // Value (`{type: "chain", coreChainLockedHeight: ..., outPoint: ...}`)
            // that round-trips through the manual `Deserialize` (which routes
            // via `RawAssetLockProof`).
            use crate::serialization::ValueConvertible;
            let chain_proof = ChainAssetLockProof::new(100, [0x42; 36]);
            let proof = AssetLockProof::Chain(chain_proof);

            let value = proof.to_object().expect("to_object");
            // The canonical `to_object` produces `type: "chain"` in the
            // wire shape. `type_from_raw_value` expects an integer-typed
            // tag (legacy shape), so it returns None on canonical output —
            // confirm via the serde Map directly instead.
            let map = value.to_map_ref().expect("map");
            assert_eq!(
                map.iter()
                    .find_map(|(k, v)| (k.as_text() == Some("$type")).then(|| v.as_text())),
                Some(Some("chain"))
            );

            let recovered =
                AssetLockProof::from_object(value).expect("from_object should round-trip");
            assert_eq!(proof, recovered);
        }

        #[test]
        fn type_from_raw_value_returns_none_for_missing_type() {
            let value = Value::Map(vec![]);
            let result = AssetLockProof::type_from_raw_value(&value);
            assert!(result.is_none());
        }

        #[test]
        fn try_from_empty_map_fails() {
            let value = Value::Map(vec![]);
            let result = AssetLockProof::try_from(&value);
            assert!(result.is_err());
        }

        #[test]
        fn try_from_value_with_unknown_key_fails() {
            let value = Value::Map(vec![(
                Value::Text("Unknown".to_string()),
                Value::Map(vec![]),
            )]);
            let result = AssetLockProof::try_from(&value);
            assert!(result.is_err());
        }
    }

    // The `try_into_value` module previously exercised the now-deleted
    // `TryInto<Value>` impls (which produced untagged `Value`). Canonical
    // `ValueConvertible::to_object` is exercised in `try_from_value` above.
}
