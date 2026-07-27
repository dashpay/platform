pub mod accessors;
pub mod fields;
mod identity_signed;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::fields::property_names::RECIPIENT_ID;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0Signable;
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

pub type IdentityCreditTransferToAddressesTransitionLatest =
    IdentityCreditTransferToAddressesTransitionV0;

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
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.identity_credit_transfer_to_addresses_state_transition"
)]
pub enum IdentityCreditTransferToAddressesTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityCreditTransferToAddressesTransitionV0),
}

impl IdentityCreditTransferToAddressesTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreditTransferToAddressesTransitionV0::default_versioned"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for IdentityCreditTransferToAddressesTransition {}

impl OptionallyAssetLockProved for IdentityCreditTransferToAddressesTransition {}

impl StateTransitionFieldTypes for IdentityCreditTransferToAddressesTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID, RECIPIENT_ID]
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
    use crate::address_funds::PlatformAddress;
    use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
    use platform_value::{platform_value, BinaryData, Identifier, Value};
    use serde_json::json;
    use std::collections::BTreeMap;

    pub(crate) fn fixture() -> IdentityCreditTransferToAddressesTransition {
        let mut recipient_addresses = BTreeMap::new();
        recipient_addresses.insert(PlatformAddress::P2pkh([0x88; 20]), 50_000u64);
        recipient_addresses.insert(PlatformAddress::P2sh([0x99; 20]), 25_000u64);

        let v0 = IdentityCreditTransferToAddressesTransitionV0 {
            identity_id: Identifier::new([0xaa; 32]),
            recipient_addresses,
            nonce: 13,
            user_fee_increase: 5,
            signature_public_key_id: 2,
            signature: BinaryData::new(vec![0xbb; 65]),
        };
        IdentityCreditTransferToAddressesTransition::V0(v0)
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields lose their size on the JSON wire (single Number type):
        //   - `nonce` is u64 (IdentityNonce), `userFeeIncrease` is u16,
        //   - `signaturePublicKeyId` is u32 (KeyID),
        //   - `recipientAddresses[].amount` is u64 (Credits).
        // The Value-path test below locks the typed variants. `Identifier` is
        // base58 in JSON HR. `BinaryData` is base64. `PlatformAddress` is hex
        // (1 type byte + 20 hash bytes).
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "identityId": "CVDFLCAjXhVWiPXH9nTCTpCgVzmDVoiPzNJYuccr1dqB",
                "recipientAddresses": [
                    {"address": "008888888888888888888888888888888888888888", "amount": 50_000},
                    {"address": "019999999999999999999999999999999999999999", "amount": 25_000},
                ],
                "nonce": 13,
                "userFeeIncrease": 5,
                "signaturePublicKeyId": 2,
                "signature": "u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7s=",
            })
        );
        let recovered =
            IdentityCreditTransferToAddressesTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // PlatformAddress is 21-byte raw bytes (1 type byte + 20-byte hash) in
        // non-HR. `nonce` is u64, `userFeeIncrease` u16, `signaturePublicKeyId` u32.
        let identity_id = Identifier::new([0xaa; 32]);
        let mut p2pkh88 = vec![0x00u8];
        p2pkh88.extend_from_slice(&[0x88u8; 20]);
        let mut p2sh99 = vec![0x01u8];
        p2sh99.extend_from_slice(&[0x99u8; 20]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "identityId": identity_id,
                "recipientAddresses": [
                    {"address": Value::Bytes(p2pkh88), "amount": 50_000u64},
                    {"address": Value::Bytes(p2sh99), "amount": 25_000u64},
                ],
                "nonce": 13u64,
                "userFeeIncrease": 5u16,
                "signaturePublicKeyId": 2u32,
                "signature": Value::Bytes(vec![0xbb; 65]),
            })
        );
        let recovered =
            IdentityCreditTransferToAddressesTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
