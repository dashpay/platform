pub mod accessors;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

use crate::state_transition::unshield_transition::v0::UnshieldTransitionV0;
use crate::state_transition::unshield_transition::v0::UnshieldTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

pub type UnshieldTransitionLatest = UnshieldTransitionV0;

use crate::identity::state_transition::OptionallyAssetLockProved;
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
    "dpp.state_transition_serialization_versions.unshield_state_transition"
)]
pub enum UnshieldTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(UnshieldTransitionV0),
}

impl OptionallyAssetLockProved for UnshieldTransition {}

impl StateTransitionFieldTypes for UnshieldTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::state_transition::unshield_transition::v0::UnshieldTransitionV0;

    fn fixture_action() -> crate::shielded::SerializedAction {
        crate::shielded::SerializedAction {
            nullifier: [0x11; 32],
            rk: [0x22; 32],
            cmx: [0x33; 32],
            encrypted_note: vec![0x44; 216],
            cv_net: [0x55; 32],
            spend_auth_sig: [0x66; 64],
        }
    }

    fn fixture() -> UnshieldTransition {
        UnshieldTransition::V0(UnshieldTransitionV0 {
            output_address: crate::address_funds::PlatformAddress::P2pkh([0xa1; 20]),
            actions: vec![fixture_action()],
            unshielding_amount: 250_000,
            anchor: [0x77; 32],
            proof: vec![0x88; 192],
            binding_signature: [0x99; 64],
        })
    }

    fn assert_v0_fields(t: &UnshieldTransition) {
        let UnshieldTransition::V0(v0) = t;
        assert_eq!(
            v0.output_address,
            crate::address_funds::PlatformAddress::P2pkh([0xa1; 20]),
            "output_address"
        );
        assert_eq!(v0.actions, vec![fixture_action()], "actions");
        assert_eq!(v0.unshielding_amount, 250_000, "unshielding_amount");
        assert_eq!(v0.anchor, [0x77; 32], "anchor");
        assert_eq!(v0.proof, vec![0x88; 192], "proof");
        assert_eq!(v0.binding_signature, [0x99; 64], "binding_signature");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = UnshieldTransition::from_json(json).expect("from_json");
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
        let recovered = UnshieldTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
