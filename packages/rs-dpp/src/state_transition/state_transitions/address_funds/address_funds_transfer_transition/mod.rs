pub mod accessors;
pub mod fields;
pub mod methods;
#[cfg(all(test, feature = "state-transition-signing"))]
mod signing_tests;
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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use platform_value::{platform_value, BinaryData, Value};
    use serde_json::json;
    use std::collections::BTreeMap;

    pub(crate) fn fixture() -> AddressFundsTransferTransition {
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

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Inputs/outputs go through helpers that emit `[{address, nonce, amount}]`
        // and `[{address, amount}]` shapes. PlatformAddress is a hex string in
        // JSON HR; BinaryData (witness signature) is base64. Sized integers
        // (nonce u32, amount u64, fee_strategy index u16, user_fee_increase u16)
        // are erased on the JSON wire — Value path locks the variants.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "inputs": [
                    {
                        "address": "00f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1",
                        "nonce": 10,
                        "amount": 800_000,
                    },
                ],
                "outputs": [
                    {
                        "address": "01f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2",
                        "amount": 700_000,
                    },
                ],
                "feeStrategy": [
                    {"$type": "reduceOutput", "index": 0},
                ],
                "userFeeIncrease": 17,
                "inputWitnesses": [
                    {
                        "$type": "p2pkh",
                        "signature": "qampqampqampqampqampqampqampqampqampqampqampqampqampqampqampqampqampqampqampqampqampqak=",
                    },
                ],
            })
        );
        let recovered = AddressFundsTransferTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // PlatformAddress / BinaryData both serialize to `Value::Bytes` in
        // non-HR. Sized ints stay sized.
        let mut input_addr = vec![0x00u8];
        input_addr.extend_from_slice(&[0xf1u8; 20]);
        let mut output_addr = vec![0x01u8];
        output_addr.extend_from_slice(&[0xf2u8; 20]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "inputs": [
                    {
                        "address": Value::Bytes(input_addr),
                        "nonce": 10u32,
                        "amount": 800_000u64,
                    },
                ],
                "outputs": [
                    {
                        "address": Value::Bytes(output_addr),
                        "amount": 700_000u64,
                    },
                ],
                "feeStrategy": [
                    {"$type": "reduceOutput", "index": 0u16},
                ],
                "userFeeIncrease": 17u16,
                "inputWitnesses": [
                    {
                        "$type": "p2pkh",
                        "signature": Value::Bytes(vec![0xa9; 65]),
                    },
                ],
            })
        );
        let recovered = AddressFundsTransferTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
