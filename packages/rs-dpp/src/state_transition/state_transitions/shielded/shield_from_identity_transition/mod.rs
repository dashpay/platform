pub mod accessors;
pub mod fields;
mod identity_signed;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

use crate::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;
use crate::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

pub type ShieldFromIdentityTransitionLatest = ShieldFromIdentityTransitionV0;

use crate::identity::state_transition::OptionallyAssetLockProved;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Moves credits from a Platform identity's balance directly into the Orchard
/// shielded pool.
///
/// The funding side is identity-signed (identity id, identity nonce, TRANSFER key),
/// exactly like `IdentityCreditTransferToAddresses`; the pool side is an
/// outputs-only Orchard bundle, exactly like `Shield`.
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
    "dpp.state_transition_serialization_versions.shield_from_identity_state_transition"
)]
pub enum ShieldFromIdentityTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(ShieldFromIdentityTransitionV0),
}

impl OptionallyAssetLockProved for ShieldFromIdentityTransition {}

impl StateTransitionFieldTypes for ShieldFromIdentityTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
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
    use crate::shielded::SerializedAction;
    use crate::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;
    use platform_value::{platform_value, BinaryData, Bytes32, Identifier};
    use serde_json::json;

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

    pub(crate) fn fixture() -> ShieldFromIdentityTransition {
        ShieldFromIdentityTransition::V0(ShieldFromIdentityTransitionV0 {
            identity_id: Identifier::new([0xaa; 32]),
            amount: 250_000,
            actions: vec![fixture_action()],
            anchor: [0x77; 32],
            proof: vec![0x88; 192],
            binding_signature: [0x99; 64],
            nonce: 13,
            user_fee_increase: 5,
            signature_public_key_id: 2,
            signature: BinaryData::new(vec![0xbb; 65]),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields lose their size on the JSON wire: `amount` (u64),
        // `nonce` (u64 IdentityNonce), `userFeeIncrease` (u16),
        // `signaturePublicKeyId` (u32 KeyID). Identifier is base58, byte fields
        // are base64.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "identityId": "CVDFLCAjXhVWiPXH9nTCTpCgVzmDVoiPzNJYuccr1dqB",
                "amount": 250_000,
                "actions": [{
                    "nullifier": "ERERERERERERERERERERERERERERERERERERERERERE=",
                    "rk": "IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI=",
                    "cmx": "MzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzM=",
                    "encryptedNote": "RERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERE",
                    "cvNet": "VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVU=",
                    "spendAuthSig": "ZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZg==",
                }],
                "anchor": "d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3c=",
                "proof": "iIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI",
                "bindingSignature": "mZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmQ==",
                "nonce": 13,
                "userFeeIncrease": 5,
                "signaturePublicKeyId": 2,
                "signature": "u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7s=",
            })
        );
        let recovered = ShieldFromIdentityTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "identityId": Identifier::new([0xaa; 32]),
                "amount": 250_000u64,
                "actions": [{
                    "nullifier": Bytes32::new([0x11; 32]),
                    "rk": Bytes32::new([0x22; 32]),
                    "cmx": Bytes32::new([0x33; 32]),
                    "encryptedNote": platform_value::Value::Bytes(vec![0x44; 216]),
                    "cvNet": Bytes32::new([0x55; 32]),
                    "spendAuthSig": platform_value::Value::Bytes(vec![0x66; 64]),
                }],
                "anchor": Bytes32::new([0x77; 32]),
                "proof": platform_value::Value::Bytes(vec![0x88; 192]),
                "bindingSignature": platform_value::Value::Bytes(vec![0x99; 64]),
                "nonce": 13u64,
                "userFeeIncrease": 5u16,
                "signaturePublicKeyId": 2u32,
                "signature": BinaryData::new(vec![0xbb; 65]),
            })
        );
        let recovered = ShieldFromIdentityTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
