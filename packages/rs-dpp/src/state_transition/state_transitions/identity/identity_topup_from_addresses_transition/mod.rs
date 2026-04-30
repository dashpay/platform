pub mod accessors;
pub mod fields;
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
use fields::*;

use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
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
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.identity_top_up_from_addresses_state_transition"
)]
pub enum IdentityTopUpFromAddressesTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityTopUpFromAddressesTransitionV0),
}

impl IdentityTopUpFromAddressesTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityTopUpFromAddressesTransition::V0(
                IdentityTopUpFromAddressesTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityTopUpFromAddressesTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for IdentityTopUpFromAddressesTransition {}

impl StateTransitionFieldTypes for IdentityTopUpFromAddressesTransition {
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

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
    use platform_value::{BinaryData, Identifier};
    use std::collections::BTreeMap;

    fn fixture() -> IdentityTopUpFromAddressesTransition {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x44; 20]), (9u32, 750_000u64));

        let v0 = IdentityTopUpFromAddressesTransitionV0 {
            inputs,
            output: Some((PlatformAddress::P2sh([0x55; 20]), 100_000)),
            identity_id: Identifier::new([0x66; 32]),
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 7,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: BinaryData::new(vec![0x77; 65]),
            }],
        };
        IdentityTopUpFromAddressesTransition::V0(v0)
    }

    fn assert_v0_fields(t: &IdentityTopUpFromAddressesTransition) {
        let IdentityTopUpFromAddressesTransition::V0(rec) = t;
        assert_eq!(rec.inputs.len(), 1, "inputs count");
        assert_eq!(
            rec.output,
            Some((PlatformAddress::P2sh([0x55; 20]), 100_000)),
            "output"
        );
        assert_eq!(rec.identity_id, Identifier::new([0x66; 32]), "identity_id");
        assert_eq!(rec.fee_strategy.len(), 1, "fee_strategy");
        assert_eq!(rec.user_fee_increase, 7, "user_fee_increase");
        assert_eq!(rec.input_witnesses.len(), 1, "input_witnesses");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered =
            IdentityTopUpFromAddressesTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered =
            IdentityTopUpFromAddressesTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        let json = fixture().to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }
}
