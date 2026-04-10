pub mod accessors;
mod fields;
#[cfg(feature = "json-conversion")]
mod json_conversion;
pub mod methods;
pub mod proved;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
pub mod v0;
#[cfg(feature = "value-conversion")]
mod value_conversion;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0Signable;
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

pub type IdentityCreateTransitionLatest = IdentityCreateTransitionV0;

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
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
    "dpp.state_transition_serialization_versions.identity_create_state_transition"
)]
pub enum IdentityCreateTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityCreateTransitionV0),
}

impl IdentityCreateTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityCreateTransition::V0(
                IdentityCreateTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreateTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl StateTransitionFieldTypes for IdentityCreateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, PUBLIC_KEYS_SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
    use crate::state_transition::{
        StateTransitionEstimatedFeeValidation, StateTransitionHasUserFeeIncrease,
        StateTransitionLike, StateTransitionOwned, StateTransitionSingleSigned,
        StateTransitionType,
    };
    use crate::version::LATEST_PLATFORM_VERSION;
    use platform_value::{BinaryData, Identifier};

    fn make_create() -> IdentityCreateTransition {
        IdentityCreateTransition::V0(IdentityCreateTransitionV0 {
            public_keys: vec![],
            asset_lock_proof: AssetLockProof::default(),
            user_fee_increase: 0,
            signature: [0u8; 65].to_vec().into(),
            identity_id: Identifier::random(),
        })
    }

    #[test]
    fn test_default_versioned() {
        let t = IdentityCreateTransition::default_versioned(LATEST_PLATFORM_VERSION)
            .expect("should create default");
        match t {
            IdentityCreateTransition::V0(_) => {}
        }
    }

    #[test]
    fn test_serialization_roundtrip() {
        let t = make_create();
        let bytes = t.serialize_to_bytes().expect("should serialize");
        let restored =
            IdentityCreateTransition::deserialize_from_bytes(&bytes).expect("should deserialize");
        assert_eq!(t, restored);
    }

    #[test]
    fn test_state_transition_like() {
        let t = make_create();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityCreate
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        let ids = t.modified_data_ids();
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn test_owner_id() {
        let t = make_create();
        match &t {
            IdentityCreateTransition::V0(v0) => {
                assert_eq!(t.owner_id(), v0.identity_id);
            }
        }
    }

    #[test]
    fn test_user_fee_increase() {
        let mut t = make_create();
        assert_eq!(t.user_fee_increase(), 0);
        t.set_user_fee_increase(5);
        assert_eq!(t.user_fee_increase(), 5);
    }

    #[test]
    fn test_single_signed() {
        let mut t = make_create();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2, 3]));
        assert_eq!(t.signature().as_slice(), &[1, 2, 3]);
        t.set_signature_bytes(vec![4, 5]);
        assert_eq!(t.signature().as_slice(), &[4, 5]);
    }

    #[test]
    fn test_accessors() {
        let t = make_create();
        assert!(t.public_keys().is_empty());
        assert_ne!(t.identity_id(), Identifier::default());
    }

    #[test]
    fn test_field_types() {
        let sig = IdentityCreateTransition::signature_property_paths();
        assert_eq!(sig.len(), 2);
        let ids = IdentityCreateTransition::identifiers_property_paths();
        assert_eq!(ids.len(), 1);
        let bin = IdentityCreateTransition::binary_property_paths();
        assert!(bin.is_empty());
    }

    #[test]
    fn test_estimated_fee() {
        let t = make_create();
        let fee = t
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calc should work");
        assert!(fee > 0);
    }

    #[test]
    fn test_into_from_v0() {
        let v0 = IdentityCreateTransitionV0::default();
        let t: IdentityCreateTransition = v0.clone().into();
        match t {
            IdentityCreateTransition::V0(inner) => assert_eq!(inner, v0),
        }
    }
}
