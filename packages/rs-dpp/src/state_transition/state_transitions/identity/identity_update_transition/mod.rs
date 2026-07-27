pub mod accessors;
pub mod fields;
mod identity_signed;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
pub mod v0;
mod v0_methods;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::identity_update_transition::fields::property_names::ADD_PUBLIC_KEYS_SIGNATURE;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;
use fields::*;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
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
    "dpp.state_transition_serialization_versions.identity_update_state_transition"
)]
pub enum IdentityUpdateTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityUpdateTransitionV0),
}

impl IdentityUpdateTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityUpdateTransition::V0(
                IdentityUpdateTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityUpdateTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl OptionallyAssetLockProved for IdentityUpdateTransition {}

impl StateTransitionFieldTypes for IdentityUpdateTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, ADD_PUBLIC_KEYS_SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![
            SIGNATURE,
            SIGNATURE_PUBLIC_KEY_ID,
            ADD_PUBLIC_KEYS_SIGNATURE,
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use crate::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
    use crate::state_transition::{
        StateTransitionEstimatedFeeValidation, StateTransitionHasUserFeeIncrease,
        StateTransitionIdentityEstimatedFeeValidation, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use crate::version::LATEST_PLATFORM_VERSION;
    use platform_value::{BinaryData, Identifier};

    fn make_update() -> IdentityUpdateTransition {
        IdentityUpdateTransition::V0(IdentityUpdateTransitionV0 {
            identity_id: Identifier::random(),
            revision: 3,
            nonce: 10,
            add_public_keys: vec![],
            disable_public_keys: vec![1],
            user_fee_increase: 2,
            signature_public_key_id: 0,
            signature: [0u8; 65].to_vec().into(),
        })
    }

    #[test]
    fn test_default_versioned() {
        let t = IdentityUpdateTransition::default_versioned(LATEST_PLATFORM_VERSION)
            .expect("should create default");
        match t {
            IdentityUpdateTransition::V0(_) => {}
        }
    }

    #[test]
    fn test_serialization_roundtrip() {
        let t = make_update();
        let bytes = t.serialize_to_bytes().expect("should serialize");
        let restored =
            IdentityUpdateTransition::deserialize_from_bytes(&bytes).expect("should deserialize");
        assert_eq!(t, restored);
    }

    #[test]
    fn test_state_transition_like() {
        let t = make_update();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityUpdate
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        let ids = t.modified_data_ids();
        assert_eq!(ids.len(), 1);
        let unique = t.unique_identifiers();
        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn test_owner_id() {
        let t = make_update();
        assert_eq!(t.owner_id(), t.identity_id());
    }

    #[test]
    fn test_user_fee_increase() {
        let mut t = make_update();
        assert_eq!(t.user_fee_increase(), 2);
        t.set_user_fee_increase(50);
        assert_eq!(t.user_fee_increase(), 50);
    }

    #[test]
    fn test_single_signed() {
        let mut t = make_update();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2]));
        assert_eq!(t.signature().as_slice(), &[1, 2]);
        t.set_signature_bytes(vec![3, 4]);
        assert_eq!(t.signature().as_slice(), &[3, 4]);
    }

    #[test]
    fn test_accessors() {
        let mut t = make_update();
        assert_eq!(t.revision(), 3);
        t.set_revision(5);
        assert_eq!(t.revision(), 5);
        assert_eq!(t.nonce(), 10);
        t.set_nonce(20);
        assert_eq!(t.nonce(), 20);
        assert!(t.public_keys_to_add().is_empty());
        assert_eq!(t.public_key_ids_to_disable(), &[1]);
        t.set_public_key_ids_to_disable(vec![2, 3]);
        assert_eq!(t.public_key_ids_to_disable(), &[2, 3]);
    }

    #[test]
    fn test_field_types() {
        let sig = IdentityUpdateTransition::signature_property_paths();
        assert_eq!(sig.len(), 3);
        let ids = IdentityUpdateTransition::identifiers_property_paths();
        assert_eq!(ids.len(), 1);
        let bin = IdentityUpdateTransition::binary_property_paths();
        assert_eq!(bin.len(), 2);
    }

    #[test]
    fn test_estimated_fee_sufficient() {
        let t = make_update();
        let fee = t
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calc should work");
        assert!(fee > 0);
        let result = t
            .validate_estimated_fee(fee + 1000, LATEST_PLATFORM_VERSION)
            .expect("validation should work");
        assert!(result.is_valid());
    }

    #[test]
    fn test_estimated_fee_insufficient() {
        let t = make_update();
        let result = t
            .validate_estimated_fee(0, LATEST_PLATFORM_VERSION)
            .expect("validation should work");
        assert!(!result.is_valid());
    }

    // Legacy `StateTransitionValueConvert` round-trip and
    // unknown-version tests deleted in Phase D step 9. The canonical
    // `JsonConvertible` / `ValueConvertible` round-trip is exercised on
    // the outer enum derive (see `json_convertible_tests` below) — these
    // tested methods that no longer exist.

    #[test]
    fn test_into_from_v0() {
        let v0 = IdentityUpdateTransitionV0::default();
        let t: IdentityUpdateTransition = v0.clone().into();
        match t {
            IdentityUpdateTransition::V0(inner) => assert_eq!(inner, v0),
        }
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

    use platform_value::{platform_value, BinaryData, Identifier};
    use serde_json::json;

    pub(crate) fn fixture() -> IdentityUpdateTransition {
        IdentityUpdateTransition::V0(IdentityUpdateTransitionV0 {
            identity_id: Identifier::new([0x55; 32]),
            revision: 3,
            nonce: 17,
            add_public_keys: vec![],
            disable_public_keys: vec![1, 2, 3],
            user_fee_increase: 4,
            signature_public_key_id: 6,
            signature: BinaryData::new(vec![0xd4; 65]),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields whose JSON wire encoding loses size info:
        // `revision` (u64), `nonce` (u64), `userFeeIncrease` (u16),
        // `signaturePublicKeyId` (u32), `disablePublicKeys` items (u32 KeyIDs).
        // The value-path assertion below uses explicit suffixes to lock variants.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "identityId": Identifier::new([0x55; 32]),
                "revision": 3,
                "nonce": 17,
                "addPublicKeys": [],
                "disablePublicKeys": [1, 2, 3],
                "userFeeIncrease": 4,
                "signaturePublicKeyId": 6,
                "signature": "1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NQ=",
            })
        );
        let recovered = IdentityUpdateTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Explicit suffixes lock in sized variants: `revision` u64, `nonce` u64,
        // `userFeeIncrease` u16, `signaturePublicKeyId` u32 (KeyID),
        // `disablePublicKeys` items u32 (KeyID).
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "identityId": Identifier::new([0x55; 32]),
                "revision": 3u64,
                "nonce": 17u64,
                "addPublicKeys": Vec::<platform_value::Value>::new(),
                "disablePublicKeys": [1u32, 2u32, 3u32],
                "userFeeIncrease": 4u16,
                "signaturePublicKeyId": 6u32,
                "signature": BinaryData::new(vec![0xd4; 65]),
            })
        );
        let recovered = IdentityUpdateTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
