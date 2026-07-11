pub mod accessors;
pub mod fields;
pub mod methods;
mod proved;
#[cfg(all(
    test,
    feature = "state-transition-signing",
    feature = "core_key_wallet",
    feature = "shielded-client"
))]
mod signing_tests;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

use crate::state_transition::shield_from_asset_lock_transition::fields::{PROOF, SIGNATURE};
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

pub type ShieldFromAssetLockTransitionLatest = ShieldFromAssetLockTransitionV0;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.shield_from_asset_lock_state_transition"
)]
pub enum ShieldFromAssetLockTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(ShieldFromAssetLockTransitionV0),
}

impl StateTransitionFieldTypes for ShieldFromAssetLockTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, PROOF]
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::shielded::SerializedAction;
    use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
    use crate::tests::fixtures::instant_asset_lock_proof_fixture;
    use platform_value::BinaryData;

    // Tier 4: `instant_asset_lock_proof_fixture` produces NON-DETERMINISTIC bytes
    // (random transaction / instantLock per run), so the full inline wire shape
    // would change between runs. We assert envelope only on `assetLockProof` and
    // a structural `assert_eq!(original, recovered)` covers that field's
    // round-trip; deterministic siblings get full literal assertions.
    pub(crate) fn fixture() -> ShieldFromAssetLockTransition {
        ShieldFromAssetLockTransition::V0(ShieldFromAssetLockTransitionV0 {
            asset_lock_proof: instant_asset_lock_proof_fixture(None, None),
            actions: vec![SerializedAction {
                nullifier: [0x11; 32],
                rk: [0x22; 32],
                cmx: [0x33; 32],
                encrypted_note: vec![0x44; 216],
                cv_net: [0x55; 32],
                spend_auth_sig: [0x66; 64],
            }],
            value_balance: 1_000_000,
            anchor: [0x77; 32],
            proof: vec![0x88; 192],
            binding_signature: [0x99; 64],
            surplus_output: None,
            signature: BinaryData::new(vec![0xab; 65]),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Envelope assertions: top-level keys + deterministic primitives.
        // `assetLockProof` is non-deterministic; only its discriminator is checked.
        let obj = json.as_object().expect("json is an object");
        assert_eq!(obj.get("$formatVersion"), Some(&serde_json::json!("0")));
        // Single action with deterministic byte fields → base64 strings.
        let actions = obj
            .get("actions")
            .and_then(|v| v.as_array())
            .expect("actions array");
        assert_eq!(actions.len(), 1);
        let act0 = actions[0].as_object().expect("action[0]");
        assert_eq!(
            act0.get("nullifier"),
            Some(&serde_json::json!(
                "ERERERERERERERERERERERERERERERERERERERERERE="
            ))
        );
        assert_eq!(
            act0.get("rk"),
            Some(&serde_json::json!(
                "IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI="
            ))
        );
        // `valueBalance` is `u64` in source. JSON erases the size on the wire —
        // value-path uses `1_000_000u64` to lock the variant.
        assert_eq!(obj.get("valueBalance"), Some(&serde_json::json!(1_000_000)));
        assert_eq!(
            obj.get("anchor"),
            Some(&serde_json::json!(
                "d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3c="
            ))
        );
        assert_eq!(
            obj.get("signature"),
            Some(&serde_json::json!(
                "q6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6s="
            ))
        );
        // assetLockProof envelope only.
        let proof = obj
            .get("assetLockProof")
            .and_then(|v| v.as_object())
            .expect("assetLockProof is an object");
        assert_eq!(proof.get("$type"), Some(&serde_json::json!("instant")));
        assert_eq!(proof.get("outputIndex"), Some(&serde_json::json!(0)));
        assert!(proof.get("instantLock").is_some_and(|v| v.is_string()));
        assert!(proof.get("transaction").is_some_and(|v| v.is_string()));
        let recovered = ShieldFromAssetLockTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Envelope assertions on the deterministic siblings; sized variants
        // locked in via explicit suffix (`1_000_000u64` for `value_balance`).
        let map = value.as_map().expect("value is a map");
        let get = |key: &str| {
            map.iter()
                .find(|(k, _)| k.as_text() == Some(key))
                .map(|(_, v)| v)
        };
        assert_eq!(
            get("$formatVersion"),
            Some(&platform_value::Value::Text("0".to_string()))
        );
        assert_eq!(
            get("valueBalance"),
            Some(&platform_value::Value::U64(1_000_000))
        );
        assert_eq!(
            get("anchor"),
            Some(&platform_value::Value::Bytes32([0x77; 32]))
        );
        assert_eq!(
            get("signature"),
            Some(&platform_value::Value::Bytes(vec![0xab; 65]))
        );
        let proof = get("assetLockProof")
            .and_then(|v| v.as_map())
            .expect("assetLockProof is a map");
        let pget = |key: &str| {
            proof
                .iter()
                .find(|(k, _)| k.as_text() == Some(key))
                .map(|(_, v)| v)
        };
        assert_eq!(
            pget("$type"),
            Some(&platform_value::Value::Text("instant".to_string()))
        );
        assert_eq!(pget("outputIndex"), Some(&platform_value::Value::U32(0)));
        assert!(pget("instantLock").is_some_and(|v| matches!(v, platform_value::Value::Bytes(_))));
        assert!(pget("transaction").is_some_and(|v| matches!(v, platform_value::Value::Bytes(_))));
        let recovered = ShieldFromAssetLockTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
