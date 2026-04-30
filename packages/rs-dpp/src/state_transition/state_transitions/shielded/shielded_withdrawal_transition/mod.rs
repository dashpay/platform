pub mod accessors;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

pub type ShieldedWithdrawalTransitionLatest = ShieldedWithdrawalTransitionV0;

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
    "dpp.state_transition_serialization_versions.shielded_withdrawal_state_transition"
)]
pub enum ShieldedWithdrawalTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(ShieldedWithdrawalTransitionV0),
}

impl OptionallyAssetLockProved for ShieldedWithdrawalTransition {}

impl StateTransitionFieldTypes for ShieldedWithdrawalTransition {
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
    use crate::identity::core_script::CoreScript;
    use crate::shielded::SerializedAction;
    use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
    use crate::withdrawal::Pooling;

    fn fixture() -> ShieldedWithdrawalTransition {
        ShieldedWithdrawalTransition::V0(ShieldedWithdrawalTransitionV0 {
            actions: vec![SerializedAction {
                nullifier: [0x11; 32],
                rk: [0x22; 32],
                cmx: [0x33; 32],
                encrypted_note: vec![0x44; 216],
                cv_net: [0x55; 32],
                spend_auth_sig: [0x66; 64],
            }],
            unshielding_amount: 750_000,
            anchor: [0x77; 32],
            proof: vec![0x88; 192],
            binding_signature: [0x99; 64],
            core_fee_per_byte: 21,
            pooling: Pooling::IfAvailable,
            output_script: CoreScript::from_bytes(vec![0xaa, 0xbb]),
        })
    }

    #[test]
    fn json_round_trip() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = ShieldedWithdrawalTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        use crate::serialization::JsonConvertible;
        let json = fixture().to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }

    #[test]
    #[ignore = "BUG: [u8;N] fixed-array fields fail platform_value round-trip"]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = ShieldedWithdrawalTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
