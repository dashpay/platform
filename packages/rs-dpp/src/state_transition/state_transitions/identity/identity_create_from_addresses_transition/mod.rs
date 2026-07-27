pub mod accessors;
mod fields;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_fee_strategy;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub type IdentityCreateFromAddressesTransitionLatest = IdentityCreateFromAddressesTransitionV0;

#[derive(
    Debug,
    Clone,
    Decode,
    Encode,
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
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.identity_create_from_addresses_state_transition"
)]
pub enum IdentityCreateFromAddressesTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityCreateFromAddressesTransitionV0),
}

impl IdentityCreateFromAddressesTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityCreateFromAddressesTransition::V0(
                IdentityCreateFromAddressesTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreateFromAddressesTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for IdentityCreateFromAddressesTransition {}

impl OptionallyAssetLockProved for IdentityCreateFromAddressesTransition {}

impl StateTransitionFieldTypes for IdentityCreateFromAddressesTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![PUBLIC_KEYS_SIGNATURE]
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
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
    use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use platform_value::{platform_value, BinaryData, Value};
    use serde_json::json;
    use std::collections::BTreeMap;

    /// Fixture with NON-DEFAULT values for every field so wire-shape
    /// assertions actually exercise data preservation.
    pub(crate) fn fixture() -> IdentityCreateFromAddressesTransition {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x11; 20]), (7u32, 1_000_000u64));
        inputs.insert(PlatformAddress::P2sh([0x22; 20]), (3u32, 500_000u64));

        let public_keys = vec![IdentityPublicKeyInCreation::V0(
            IdentityPublicKeyInCreationV0 {
                id: 5,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MASTER,
                contract_bounds: None,
                read_only: false,
                data: BinaryData::new(vec![0xab; 33]),
                signature: BinaryData::new(vec![0xcd; 65]),
            },
        )];

        let v0 = IdentityCreateFromAddressesTransitionV0 {
            public_keys,
            inputs,
            output: Some((PlatformAddress::P2pkh([0x33; 20]), 250_000)),
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 42,
            input_witnesses: vec![
                AddressWitness::P2pkh {
                    signature: BinaryData::new(vec![0xee; 65]),
                },
                AddressWitness::P2sh {
                    redeem_script: BinaryData::new(vec![0xff; 30]),
                    signatures: vec![BinaryData::new(vec![0x12; 65])],
                },
            ],
        };
        IdentityCreateFromAddressesTransition::V0(v0)
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields lose their size on the JSON wire (single Number type):
        //   - public key `id` is u32, `type`/`purpose`/`securityLevel` are u8 enums,
        //   - `inputs[].nonce` is u32, `inputs[].amount` / `output.amount` are u64,
        //   - `feeStrategy[].index` is u16, `userFeeIncrease` is u16.
        // The Value-path test below locks the typed variants. `BinaryData` is
        // base64 in JSON, `Value::Bytes` in non-HR. `PlatformAddress` is hex
        // string in JSON (1 type byte + 20 hash bytes), raw bytes in Value.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "publicKeys": [
                    {
                        "$formatVersion": "0",
                        "id": 5,
                        "type": 0,
                        "purpose": 0,
                        "securityLevel": 0,
                        "contractBounds": null,
                        "readOnly": false,
                        "data": "q6urq6urq6urq6urq6urq6urq6urq6urq6urq6urq6ur",
                        "signature": "zc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc3Nzc0=",
                    },
                ],
                "inputs": [
                    {"address": "001111111111111111111111111111111111111111", "nonce": 7, "amount": 1_000_000},
                    {"address": "012222222222222222222222222222222222222222", "nonce": 3, "amount": 500_000},
                ],
                "output": {"address": "003333333333333333333333333333333333333333", "amount": 250_000},
                "feeStrategy": [{"$type": "deductFromInput", "index": 0}],
                "userFeeIncrease": 42,
                "inputWitnesses": [
                    {
                        "$type": "p2pkh",
                        "signature": "7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u4=",
                    },
                    {
                        "$type": "p2sh",
                        "signatures": [
                            "EhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhISEhI=",
                        ],
                        "redeemScript": "////////////////////////////////////////",
                    },
                ],
            })
        );
        let recovered = IdentityCreateFromAddressesTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // PlatformAddress emits 21-byte raw bytes (1 type byte + 20-byte hash) in
        // non-HR. KeyType/Purpose/SecurityLevel are #[repr(u8)] with u8 wire.
        // `id` is u32 (KeyID), `nonce` is u32 (AddressNonce), `amount` is u64
        // (Credits), `index` is u16, `userFeeIncrease` is u16.
        let mut p2pkh11_bytes = vec![0x00u8];
        p2pkh11_bytes.extend_from_slice(&[0x11u8; 20]);
        let mut p2sh22_bytes = vec![0x01u8];
        p2sh22_bytes.extend_from_slice(&[0x22u8; 20]);
        let mut p2pkh33_bytes = vec![0x00u8];
        p2pkh33_bytes.extend_from_slice(&[0x33u8; 20]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "publicKeys": [
                    {
                        "$formatVersion": "0",
                        "id": 5u32,
                        "type": 0u8,
                        "purpose": 0u8,
                        "securityLevel": 0u8,
                        "contractBounds": Value::Null,
                        "readOnly": false,
                        "data": Value::Bytes(vec![0xab; 33]),
                        "signature": Value::Bytes(vec![0xcd; 65]),
                    },
                ],
                "inputs": [
                    {"address": Value::Bytes(p2pkh11_bytes), "nonce": 7u32, "amount": 1_000_000u64},
                    {"address": Value::Bytes(p2sh22_bytes), "nonce": 3u32, "amount": 500_000u64},
                ],
                "output": {"address": Value::Bytes(p2pkh33_bytes), "amount": 250_000u64},
                "feeStrategy": [{"$type": "deductFromInput", "index": 0u16}],
                "userFeeIncrease": 42u16,
                "inputWitnesses": [
                    {
                        "$type": "p2pkh",
                        "signature": Value::Bytes(vec![0xee; 65]),
                    },
                    {
                        "$type": "p2sh",
                        "signatures": [Value::Bytes(vec![0x12; 65])],
                        "redeemScript": Value::Bytes(vec![0xff; 30]),
                    },
                ],
            })
        );
        let recovered =
            IdentityCreateFromAddressesTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
