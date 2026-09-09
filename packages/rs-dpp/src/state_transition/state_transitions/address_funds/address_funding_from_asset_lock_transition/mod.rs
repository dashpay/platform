pub mod accessors;
mod fields;
pub mod methods;
mod proved;
#[cfg(all(test, feature = "state-transition-signing"))]
mod signing_tests;
mod state_transition_estimated_fee_validation;
mod state_transition_fee_strategy;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

pub use state_transition_estimated_fee_validation::calculate_address_funding_from_asset_lock_min_required_fee;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub type AddressFundingFromAssetLockTransitionLatest = AddressFundingFromAssetLockTransitionV0;

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
    "dpp.state_transition_serialization_versions.address_funding_from_asset_lock_state_transition"
)]
pub enum AddressFundingFromAssetLockTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(AddressFundingFromAssetLockTransitionV0),
}

impl AddressFundingFromAssetLockTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .address_funding_from_asset_lock_state_transition
            .default_current_version
        {
            0 => Ok(AddressFundingFromAssetLockTransition::V0(
                AddressFundingFromAssetLockTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "AddressFundingFromAssetLockTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for AddressFundingFromAssetLockTransition {}

impl StateTransitionFieldTypes for AddressFundingFromAssetLockTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
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
    use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
    use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
    use dashcore::OutPoint;
    use platform_value::{platform_value, BinaryData, Value};
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::str::FromStr;

    pub(crate) fn fixture() -> AddressFundingFromAssetLockTransition {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xa1; 20]), (4u32, 600_000u64));

        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2pkh([0xb2; 20]), Some(400_000u64));
        outputs.insert(PlatformAddress::P2sh([0xc3; 20]), None); // remainder

        let asset_lock_proof = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 12345,
            out_point: OutPoint::from_str(
                "0000000000000000000000000000000000000000000000000000000000000001:1",
            )
            .expect("outpoint"),
        });

        let v0 = AddressFundingFromAssetLockTransitionV0 {
            asset_lock_proof,
            inputs,
            outputs,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 11,
            signature: BinaryData::new(vec![0xd4; 65]),
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: BinaryData::new(vec![0xe5; 65]),
            }],
        };
        AddressFundingFromAssetLockTransition::V0(v0)
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `assetLockProof` is internally tagged `{type: "chain", ...}` with
        // `outPoint` rendered as the human-readable `txid:vout` string in JSON
        // (Value-path uses the structured `{txid: Bytes32, vout: U32}` form).
        // Sized-int fields (heights, nonces, indexes, fee numbers) lose their
        // size on the JSON wire — Value path locks the typed variants.
        // BinaryData / PlatformAddress / outputs[].amount notes follow the
        // pattern in the credit-withdrawal test next door.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "assetLockProof": {
                    "$type": "chain",
                    "coreChainLockedHeight": 12345,
                    "outPoint": "0000000000000000000000000000000000000000000000000000000000000001:1",
                },
                "inputs": [
                    {
                        "address": "00a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1",
                        "nonce": 4,
                        "amount": 600_000,
                    },
                ],
                "outputs": [
                    {
                        "address": "00b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2",
                        "amount": 400_000,
                    },
                    {
                        "address": "01c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3",
                        "amount": null,
                    },
                ],
                "feeStrategy": [
                    {"$type": "deductFromInput", "index": 0},
                ],
                "userFeeIncrease": 11,
                "signature": "1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NQ=",
                "inputWitnesses": [
                    {
                        "$type": "p2pkh",
                        "signature": "5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eU=",
                    },
                ],
            })
        );
        let recovered = AddressFundingFromAssetLockTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `outPoint` is the structured `{txid: Bytes32, vout: U32}` form here
        // (the JSON path collapses it to `"txid:vout"`). PlatformAddress
        // serializes to `Value::Bytes` (21 bytes: 1 type byte + 20 hash) in
        // non-HR; BinaryData also serializes to `Value::Bytes`. Sized integers
        // stay sized: `coreChainLockedHeight`, `nonce`, `coreFeePerByte` are
        // U32; credit amounts U64; `userFeeIncrease`/`feeStrategy.index` U16.
        let mut input_addr = vec![0x00u8];
        input_addr.extend_from_slice(&[0xa1u8; 20]);
        let mut output_pkh = vec![0x00u8];
        output_pkh.extend_from_slice(&[0xb2u8; 20]);
        let mut output_sh = vec![0x01u8];
        output_sh.extend_from_slice(&[0xc3u8; 20]);
        let mut txid_bytes = [0u8; 32];
        txid_bytes[0] = 1;
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "assetLockProof": {
                    "$type": "chain",
                    "coreChainLockedHeight": 12345u32,
                    "outPoint": {
                        "txid": Value::Bytes32(txid_bytes),
                        "vout": 1u32,
                    },
                },
                "inputs": [
                    {
                        "address": Value::Bytes(input_addr),
                        "nonce": 4u32,
                        "amount": 600_000u64,
                    },
                ],
                "outputs": [
                    {
                        "address": Value::Bytes(output_pkh),
                        "amount": 400_000u64,
                    },
                    {
                        "address": Value::Bytes(output_sh),
                        "amount": Value::Null,
                    },
                ],
                "feeStrategy": [
                    {"$type": "deductFromInput", "index": 0u16},
                ],
                "userFeeIncrease": 11u16,
                "signature": Value::Bytes(vec![0xd4; 65]),
                "inputWitnesses": [
                    {
                        "$type": "p2pkh",
                        "signature": Value::Bytes(vec![0xe5; 65]),
                    },
                ],
            })
        );
        let recovered =
            AddressFundingFromAssetLockTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
