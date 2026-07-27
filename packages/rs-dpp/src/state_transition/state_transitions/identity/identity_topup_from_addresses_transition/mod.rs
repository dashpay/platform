pub mod accessors;
pub mod fields;
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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
    use platform_value::{platform_value, BinaryData, Identifier, Value};
    use serde_json::json;
    use std::collections::BTreeMap;

    pub(crate) fn fixture() -> IdentityTopUpFromAddressesTransition {
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

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields lose their size on the JSON wire (single Number type):
        //   - `inputs[].nonce` is u32, `inputs[].amount` / `output.amount` are u64,
        //   - `feeStrategy[].index` is u16, `userFeeIncrease` is u16.
        // The Value-path test below locks the typed variants. `Identifier` is
        // base58 in JSON HR. `BinaryData` is base64. `PlatformAddress` is hex
        // (1 type byte + 20 hash bytes).
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "inputs": [
                    {"address": "004444444444444444444444444444444444444444", "nonce": 9, "amount": 750_000},
                ],
                "output": {"address": "015555555555555555555555555555555555555555", "amount": 100_000},
                "identityId": "7tj9biW3KRJ7EEWmVUGigHiouCTXhV2dzcyvwma7Cyu7",
                "feeStrategy": [{"$type": "deductFromInput", "index": 0}],
                "userFeeIncrease": 7,
                "inputWitnesses": [
                    {
                        "$type": "p2pkh",
                        "signature": "d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3c=",
                    },
                ],
            })
        );
        let recovered = IdentityTopUpFromAddressesTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let identity_id = Identifier::new([0x66; 32]);
        let mut p2pkh44 = vec![0x00u8];
        p2pkh44.extend_from_slice(&[0x44u8; 20]);
        let mut p2sh55 = vec![0x01u8];
        p2sh55.extend_from_slice(&[0x55u8; 20]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "inputs": [
                    {"address": Value::Bytes(p2pkh44), "nonce": 9u32, "amount": 750_000u64},
                ],
                "output": {"address": Value::Bytes(p2sh55), "amount": 100_000u64},
                "identityId": identity_id,
                "feeStrategy": [{"$type": "deductFromInput", "index": 0u16}],
                "userFeeIncrease": 7u16,
                "inputWitnesses": [
                    {
                        "$type": "p2pkh",
                        "signature": Value::Bytes(vec![0x77; 65]),
                    },
                ],
            })
        );
        let recovered =
            IdentityTopUpFromAddressesTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
