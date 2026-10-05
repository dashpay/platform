pub mod accessors;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
pub mod v1;
mod version;

use crate::state_transition::shield_transition::v0::ShieldTransitionV0;
use crate::state_transition::shield_transition::v0::ShieldTransitionV0Signable;
use crate::state_transition::shield_transition::v1::{
    ShieldTransitionV1, ShieldTransitionV1Signable,
};
use crate::state_transition::StateTransitionFieldTypes;

pub type ShieldTransitionLatest = ShieldTransitionV1;

use crate::identity::state_transition::OptionallyAssetLockProved;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
    DecodeUntrusted,
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
    /// Binds the bundle to its kind and funding addresses. Admitted only from protocol version 14.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "1"))]
    V1(ShieldTransitionV1),
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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::dashcore::hashes::{sha256, Hash};
    use crate::serialization::{
        JsonConvertible, PlatformDeserializableUntrusted, PlatformSerializable, Signable,
        ValueConvertible,
    };
    use crate::shielded::SerializedAction;
    use crate::state_transition::shield_transition::v0::ShieldTransitionV0;
    use crate::state_transition::StateTransition;
    use platform_value::{platform_value, BinaryData, Bytes32};
    use platform_version::version::PlatformVersion;
    use serde_json::json;
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

    pub(crate) fn fixture() -> ShieldTransition {
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

    fn bound_fixture() -> ShieldTransition {
        let ShieldTransition::V0(v0) = fixture() else {
            panic!("historical fixture");
        };
        ShieldTransition::V1(ShieldTransitionV1 {
            inputs: v0.inputs,
            actions: v0.actions,
            amount: v0.amount,
            anchor: v0.anchor,
            proof: v0.proof,
            binding_signature: v0.binding_signature,
            fee_strategy: v0.fee_strategy,
            user_fee_increase: v0.user_fee_increase,
            input_witnesses: v0.input_witnesses,
        })
    }

    #[test]
    fn should_separate_shield_formats_at_protocol_activation() {
        for (pv, format) in [
            (PlatformVersion::get(12).expect("PV12"), 0),
            (PlatformVersion::get(13).expect("PV13"), 0),
            (PlatformVersion::latest(), 1),
        ] {
            let bounds = &pv
                .dpp
                .state_transition_serialization_versions
                .shield_state_transition;
            assert_eq!(
                (
                    bounds.min_version,
                    bounds.max_version,
                    bounds.default_current_version
                ),
                (format, format, format)
            );
            for shield in [fixture(), bound_fixture()] {
                let supported = shield.feature_version() == format;
                let transition = StateTransition::Shield(shield);
                let bytes = transition.serialize_to_bytes().expect("wire bytes");
                assert_eq!(
                    StateTransition::deserialize_from_bytes_untrusted(&bytes)
                        .expect("generic decoding"),
                    transition
                );
                assert_eq!(
                    StateTransition::deserialize_from_bytes_untrusted_exact_in_version(&bytes, pv)
                        .is_ok(),
                    supported
                );
            }
        }
    }

    #[test]
    fn should_include_the_shield_format_in_address_signing_bytes() {
        let legacy = StateTransition::Shield(fixture());
        let bound = StateTransition::Shield(bound_fixture());
        for (before, after) in [
            (
                legacy.serialize_to_bytes().expect("legacy"),
                bound.serialize_to_bytes().expect("bound"),
            ),
            (
                legacy.signable_bytes().expect("legacy signing bytes"),
                bound.signable_bytes().expect("bound signing bytes"),
            ),
        ] {
            assert_eq!(before.len(), after.len());
            assert_eq!(
                before.iter().zip(&after).filter(|(a, b)| a != b).count(),
                1,
                "identical fields must differ only in the signed format discriminant"
            );
        }
    }

    #[test]
    fn should_round_trip_bound_shields_without_losing_the_format() {
        let original = bound_fixture();
        let json = original.to_json().expect("JSON");
        assert_eq!(json["$formatVersion"], "1");
        assert_eq!(
            ShieldTransition::from_json(json).expect("JSON round trip"),
            original
        );
        let value = original.to_object().expect("Value");
        assert_eq!(
            ShieldTransition::from_object(value).expect("Value round trip"),
            original
        );
    }

    #[test]
    fn should_preserve_historical_shield_wire_and_signable_bytes() {
        let transition = StateTransition::Shield(fixture());
        let wire = transition
            .serialize_to_bytes()
            .expect("historical wire bytes");
        let signable = transition
            .signable_bytes()
            .expect("historical signing bytes");
        assert_eq!(
            sha256::Hash::hash(&wire).to_string(),
            "cb3bd0211bf65d650f543cd26c41e8ea8a3f65966a7d724da0dbb84c6d954171"
        );
        assert_eq!(
            sha256::Hash::hash(&signable).to_string(),
            "e4d9d6efea0f33c18b114c9250322c190af2eec35d1478ebadb59db243a6b45a"
        );
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields whose JSON wire encoding loses size info:
        // `inputs[].nonce` (u32 AddressNonce), `inputs[].amount` (u64),
        // `amount` (u64), `feeStrategy[].index` (u16),
        // `userFeeIncrease` (u16). PlatformAddress → hex string in HR / 21 bytes
        // non-HR; AddressWitness uses externally-tagged `{type, signature}`.
        // BTreeMap<PlatformAddress, (u32, u64)> serializes as array of
        // `{address, nonce, amount}` objects, NOT a JSON map. The value-path
        // assertion locks all sized variants via explicit suffixes.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "inputs": [{
                    "address": "00a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1",
                    "nonce": 3,
                    "amount": 500_000,
                }],
                "actions": [{
                    "nullifier": "ERERERERERERERERERERERERERERERERERERERERERE=",
                    "rk": "IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI=",
                    "cmx": "MzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzM=",
                    "encryptedNote": "RERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERE",
                    "cvNet": "VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVU=",
                    "spendAuthSig": "ZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZg==",
                }],
                "amount": 250_000,
                "anchor": "d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3c=",
                "proof": "iIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI",
                "bindingSignature": "mZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmQ==",
                "feeStrategy": [{"$type": "deductFromInput", "index": 0}],
                "userFeeIncrease": 5,
                "inputWitnesses": [{
                    "$type": "p2pkh",
                    "signature": "qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqo=",
                }],
            })
        );
        let recovered = ShieldTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Explicit suffixes lock in sized variants: `inputs[].nonce` u32
        // (AddressNonce), `inputs[].amount` u64, `amount` u64,
        // `feeStrategy[].index` u16, `userFeeIncrease` u16.
        // PlatformAddress non-HR → 21-byte `Value::Bytes` (P2pkh type byte 0x00).
        let mut address_bytes = vec![0x00];
        address_bytes.extend(vec![0xa1; 20]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "inputs": [{
                    "address": platform_value::Value::Bytes(address_bytes),
                    "nonce": 3u32,
                    "amount": 500_000u64,
                }],
                "actions": [{
                    "nullifier": Bytes32::new([0x11; 32]),
                    "rk": Bytes32::new([0x22; 32]),
                    "cmx": Bytes32::new([0x33; 32]),
                    "encryptedNote": platform_value::Value::Bytes(vec![0x44; 216]),
                    "cvNet": Bytes32::new([0x55; 32]),
                    "spendAuthSig": platform_value::Value::Bytes(vec![0x66; 64]),
                }],
                "amount": 250_000u64,
                "anchor": Bytes32::new([0x77; 32]),
                "proof": platform_value::Value::Bytes(vec![0x88; 192]),
                "bindingSignature": platform_value::Value::Bytes(vec![0x99; 64]),
                "feeStrategy": [{"$type": "deductFromInput", "index": 0u16}],
                "userFeeIncrease": 5u16,
                "inputWitnesses": [{
                    "$type": "p2pkh",
                    "signature": BinaryData::new(vec![0xaa; 65]),
                }],
            })
        );
        let recovered = ShieldTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
