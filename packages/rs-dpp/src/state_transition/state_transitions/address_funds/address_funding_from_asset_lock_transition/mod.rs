pub mod accessors;
mod fields;
#[cfg(feature = "json-conversion")]
mod json_conversion;
pub mod methods;
mod proved;
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

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
    use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
    use dashcore::OutPoint;
    use platform_value::{BinaryData, Identifier};
    use std::collections::BTreeMap;
    use std::str::FromStr;

    fn fixture() -> AddressFundingFromAssetLockTransition {
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

    fn assert_v0_fields(t: &AddressFundingFromAssetLockTransition) {
        let AddressFundingFromAssetLockTransition::V0(rec) = t;
        match &rec.asset_lock_proof {
            AssetLockProof::Chain(c) => {
                assert_eq!(c.core_chain_locked_height, 12345, "asset_lock_proof.height");
            }
            other => panic!("expected Chain proof, got {:?}", other),
        }
        assert_eq!(rec.inputs.len(), 1, "inputs count");
        assert_eq!(rec.outputs.len(), 2, "outputs count");
        assert_eq!(rec.fee_strategy.len(), 1, "fee_strategy");
        assert_eq!(rec.user_fee_increase, 11, "user_fee_increase");
        assert_eq!(rec.signature, BinaryData::new(vec![0xd4; 65]), "signature");
        assert_eq!(rec.input_witnesses.len(), 1, "input_witnesses");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered =
            AddressFundingFromAssetLockTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered =
            AddressFundingFromAssetLockTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        let json = fixture().to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }
}
