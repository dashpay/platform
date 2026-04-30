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

#[cfg(all(test, feature = "json-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;

    fn fixture() -> AddressFundingFromAssetLockTransition {
        AddressFundingFromAssetLockTransition::V0(
            AddressFundingFromAssetLockTransitionV0::default(),
        )
    }

    #[test]
    fn json_round_trip() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered =
            AddressFundingFromAssetLockTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        let json = fixture().to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }
}
