pub mod accessors;
pub mod fields;
#[cfg(feature = "json-conversion")]
mod json_conversion;
pub mod methods;
#[cfg(all(test, feature = "state-transition-signing"))]
mod signing_tests;
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
use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub type UTXOTransferTransitionLatest = AddressFundsTransferTransitionV0;

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
    "dpp.state_transition_serialization_versions.address_funds_transfer_state_transition"
)]
pub enum AddressFundsTransferTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(AddressFundsTransferTransitionV0),
}

impl AddressFundsTransferTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .address_funds
            .address_funds_transition_default_version
        {
            0 => Ok(AddressFundsTransferTransition::V0(
                AddressFundsTransferTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "AddressFundsTransferTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for AddressFundsTransferTransition {}

impl OptionallyAssetLockProved for AddressFundsTransferTransition {}

impl StateTransitionFieldTypes for AddressFundsTransferTransition {
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
    use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use platform_value::BinaryData;
    use std::collections::BTreeMap;

    fn fixture() -> AddressFundsTransferTransition {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xf1; 20]), (10u32, 800_000u64));

        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2sh([0xf2; 20]), 700_000u64);

        let v0 = AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            fee_strategy: vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            user_fee_increase: 17,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: BinaryData::new(vec![0xa9; 65]),
            }],
        };
        AddressFundsTransferTransition::V0(v0)
    }

    fn assert_v0_fields(t: &AddressFundsTransferTransition) {
        let AddressFundsTransferTransition::V0(rec) = t;
        assert_eq!(rec.inputs.len(), 1, "inputs count");
        assert_eq!(rec.outputs.len(), 1, "outputs count");
        assert_eq!(rec.fee_strategy.len(), 1, "fee_strategy");
        assert_eq!(rec.user_fee_increase, 17, "user_fee_increase");
        assert_eq!(rec.input_witnesses.len(), 1, "input_witnesses");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = AddressFundsTransferTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = AddressFundsTransferTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        let json = fixture().to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }
}
