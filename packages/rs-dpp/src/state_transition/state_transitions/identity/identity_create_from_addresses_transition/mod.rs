pub mod accessors;
mod fields;
#[cfg(feature = "json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_fee_strategy;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
#[cfg(feature = "value-conversion")]
mod value_conversion;
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

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::address_funds::{
        AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress,
    };
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
    use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use platform_value::BinaryData;
    use std::collections::BTreeMap;

    /// Fixture with NON-DEFAULT values for every field so per-property
    /// assertions actually exercise data preservation.
    fn fixture() -> IdentityCreateFromAddressesTransition {
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

    fn assert_v0_fields(t: &IdentityCreateFromAddressesTransition) {
        let IdentityCreateFromAddressesTransition::V0(rec) = t;
        // 6-field per-property assertion
        assert_eq!(rec.public_keys.len(), 1, "public_keys count");
        assert_eq!(rec.inputs.len(), 2, "inputs count");
        assert_eq!(
            rec.output,
            Some((PlatformAddress::P2pkh([0x33; 20]), 250_000)),
            "output"
        );
        assert_eq!(rec.fee_strategy.len(), 1, "fee_strategy count");
        assert_eq!(rec.user_fee_increase, 42, "user_fee_increase");
        assert_eq!(rec.input_witnesses.len(), 2, "input_witnesses count");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered =
            IdentityCreateFromAddressesTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered =
            IdentityCreateFromAddressesTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        let json = fixture().to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }
}
