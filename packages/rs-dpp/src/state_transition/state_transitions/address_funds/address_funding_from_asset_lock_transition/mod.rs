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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition::{FeatureVersioned, StateTransitionLike, StateTransitionType};

    #[test]
    fn default_versioned_succeeds_on_latest() {
        let pv = PlatformVersion::latest();
        let result = AddressFundingFromAssetLockTransition::default_versioned(pv);
        assert!(result.is_ok());
        let transition = result.unwrap();
        assert!(matches!(
            transition,
            AddressFundingFromAssetLockTransition::V0(_)
        ));
    }

    #[test]
    fn field_types_signature_property_paths() {
        let paths = AddressFundingFromAssetLockTransition::signature_property_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], SIGNATURE);
    }

    #[test]
    fn field_types_identifiers_is_empty() {
        assert!(AddressFundingFromAssetLockTransition::identifiers_property_paths().is_empty());
    }

    #[test]
    fn field_types_binary_is_empty() {
        assert!(AddressFundingFromAssetLockTransition::binary_property_paths().is_empty());
    }

    #[test]
    fn feature_version_is_zero() {
        let pv = PlatformVersion::latest();
        let t = AddressFundingFromAssetLockTransition::default_versioned(pv).unwrap();
        assert_eq!(t.feature_version(), 0);
    }

    #[test]
    fn state_transition_type_through_enum() {
        let pv = PlatformVersion::latest();
        let t = AddressFundingFromAssetLockTransition::default_versioned(pv).unwrap();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::AddressFundingFromAssetLock
        );
    }
}
