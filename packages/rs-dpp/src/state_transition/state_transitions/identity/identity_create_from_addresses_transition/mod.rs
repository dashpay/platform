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

#[cfg(all(test, feature = "json-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;

    fn fixture() -> IdentityCreateFromAddressesTransition {
        IdentityCreateFromAddressesTransition::V0(
            IdentityCreateFromAddressesTransitionV0::default(),
        )
    }

    #[test]
    fn json_round_trip() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered =
            IdentityCreateFromAddressesTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        let json = fixture().to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }
}
