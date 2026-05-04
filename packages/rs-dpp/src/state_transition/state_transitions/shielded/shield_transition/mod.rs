pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

use crate::state_transition::shield_transition::v0::ShieldTransitionV0;
use crate::state_transition::shield_transition::v0::ShieldTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

pub type ShieldTransitionLatest = ShieldTransitionV0;

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
    "dpp.state_transition_serialization_versions.shield_state_transition"
)]
pub enum ShieldTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(ShieldTransitionV0),
}

impl OptionallyAssetLockProved for ShieldTransition {}

impl StateTransitionFieldTypes for ShieldTransition {
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
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::shielded::SerializedAction;
    use crate::state_transition::shield_transition::v0::ShieldTransitionV0;
    use platform_value::BinaryData;
    use std::collections::BTreeMap;

    fn fixture_action() -> SerializedAction {
        SerializedAction {
            nullifier: [0x11; 32],
            rk: [0x22; 32],
            cmx: [0x33; 32],
            encrypted_note: vec![0x44; 216],
            cv_net: [0x55; 32],
            spend_auth_sig: [0x66; 64],
        }
    }

    fn fixture() -> ShieldTransition {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xa1; 20]), (3u32, 500_000u64));
        ShieldTransition::V0(ShieldTransitionV0 {
            inputs,
            actions: vec![fixture_action()],
            amount: 250_000,
            anchor: [0x77; 32],
            proof: vec![0x88; 192],
            binding_signature: [0x99; 64],
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 5,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: BinaryData::new(vec![0xaa; 65]),
            }],
        })
    }

    fn assert_v0_fields(t: &ShieldTransition) {
        let ShieldTransition::V0(v0) = t;
        assert_eq!(v0.inputs.len(), 1, "inputs.len");
        assert_eq!(
            v0.inputs.get(&PlatformAddress::P2pkh([0xa1; 20])),
            Some(&(3u32, 500_000u64)),
            "inputs entry"
        );
        assert_eq!(v0.actions, vec![fixture_action()], "actions");
        assert_eq!(v0.amount, 250_000, "amount");
        assert_eq!(v0.anchor, [0x77; 32], "anchor");
        assert_eq!(v0.proof, vec![0x88; 192], "proof");
        assert_eq!(v0.binding_signature, [0x99; 64], "binding_signature");
        assert_eq!(
            v0.fee_strategy,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            "fee_strategy"
        );
        assert_eq!(v0.user_fee_increase, 5, "user_fee_increase");
        assert_eq!(v0.input_witnesses.len(), 1, "input_witnesses.len");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = ShieldTransition::from_json(json).expect("from_json");
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
        let recovered = ShieldTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
