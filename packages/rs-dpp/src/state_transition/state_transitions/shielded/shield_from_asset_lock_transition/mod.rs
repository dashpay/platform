pub mod fields;
pub mod methods;
mod proved;
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

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::shielded::SerializedAction;
    use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
    use crate::tests::fixtures::instant_asset_lock_proof_fixture;
    use platform_value::BinaryData;

    fn fixture() -> ShieldFromAssetLockTransition {
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
            signature: BinaryData::new(vec![0xab; 65]),
        })
    }

    fn assert_v0_fields(t: &ShieldFromAssetLockTransition) {
        // Hardcoded expected values per the fixture above. Note:
        // `asset_lock_proof` is intentionally absent from this helper —
        // `instant_asset_lock_proof_fixture` is non-deterministic (random
        // one-time-private-key per call), so there is no stable expected
        // value to assert against. The structural `assert_eq!(original,
        // recovered)` in each test still covers that field's round-trip.
        let ShieldFromAssetLockTransition::V0(v0) = t;
        assert_eq!(v0.actions.len(), 1, "actions.len");
        assert_eq!(v0.actions[0].nullifier, [0x11; 32], "actions[0].nullifier");
        assert_eq!(v0.actions[0].rk, [0x22; 32], "actions[0].rk");
        assert_eq!(v0.actions[0].cmx, [0x33; 32], "actions[0].cmx");
        assert_eq!(
            v0.actions[0].encrypted_note,
            vec![0x44; 216],
            "actions[0].encrypted_note"
        );
        assert_eq!(v0.actions[0].cv_net, [0x55; 32], "actions[0].cv_net");
        assert_eq!(
            v0.actions[0].spend_auth_sig, [0x66; 64],
            "actions[0].spend_auth_sig"
        );
        assert_eq!(v0.value_balance, 1_000_000, "value_balance");
        assert_eq!(v0.anchor, [0x77; 32], "anchor");
        assert_eq!(v0.proof, vec![0x88; 192], "proof");
        assert_eq!(v0.binding_signature, [0x99; 64], "binding_signature");
        assert_eq!(v0.signature, BinaryData::new(vec![0xab; 65]), "signature");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = ShieldFromAssetLockTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        use crate::serialization::JsonConvertible;
        let json = fixture().to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = ShieldFromAssetLockTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
