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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::identity::core_script::CoreScript;
    use crate::shielded::SerializedAction;
    use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
    use crate::withdrawal::Pooling;
    use platform_value::{platform_value, BinaryData, Bytes32};
    use serde_json::json;

    pub(crate) fn fixture() -> ShieldedWithdrawalTransition {
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
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields whose JSON wire encoding loses size info:
        // `unshieldingAmount` (u64), `coreFeePerByte` (u32). `pooling` uses
        // `pooling_serde` which emits the camelCase name in HR and u8 in non-HR.
        // `outputScript` is base64 in HR (CoreScript Serialize) and bytes in non-HR.
        // SerializedAction 32-byte fields → base64 in HR, `Value::Bytes32` non-HR.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "actions": [{
                    "nullifier": "ERERERERERERERERERERERERERERERERERERERERERE=",
                    "rk": "IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI=",
                    "cmx": "MzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzM=",
                    "encryptedNote": "RERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERE",
                    "cvNet": "VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVU=",
                    "spendAuthSig": "ZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZg==",
                }],
                "unshieldingAmount": 750_000,
                "anchor": "d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3c=",
                "proof": "iIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI",
                "bindingSignature": "mZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmQ==",
                "coreFeePerByte": 21,
                "pooling": "ifAvailable",
                "outputScript": "qrs=",
            })
        );
        let recovered = ShieldedWithdrawalTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Explicit suffixes lock in sized variants: `unshieldingAmount` u64,
        // `coreFeePerByte` u32. `pooling` non-HR path emits the u8 discriminant
        // (`Pooling::IfAvailable as u8 == 1`). `outputScript` non-HR → bytes.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "actions": [{
                    "nullifier": Bytes32::new([0x11; 32]),
                    "rk": Bytes32::new([0x22; 32]),
                    "cmx": Bytes32::new([0x33; 32]),
                    "encryptedNote": platform_value::Value::Bytes(vec![0x44; 216]),
                    "cvNet": Bytes32::new([0x55; 32]),
                    "spendAuthSig": platform_value::Value::Bytes(vec![0x66; 64]),
                }],
                "unshieldingAmount": 750_000u64,
                "anchor": Bytes32::new([0x77; 32]),
                "proof": platform_value::Value::Bytes(vec![0x88; 192]),
                "bindingSignature": platform_value::Value::Bytes(vec![0x99; 64]),
                "coreFeePerByte": 21u32,
                "pooling": 1u8,
                "outputScript": BinaryData::new(vec![0xaa, 0xbb]),
            })
        );
        let recovered = ShieldedWithdrawalTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
