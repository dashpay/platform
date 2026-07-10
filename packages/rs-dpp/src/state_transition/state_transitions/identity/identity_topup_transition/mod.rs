pub mod accessors;
pub mod fields;
pub mod methods;
pub mod proved;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
pub mod v0;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use fields::*;

use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

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
    "dpp.state_transition_serialization_versions.identity_top_up_state_transition"
)]
pub enum IdentityTopUpTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityTopUpTransitionV0),
}

impl IdentityTopUpTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityTopUpTransition::V0(
                IdentityTopUpTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityTopUpTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl StateTransitionFieldTypes for IdentityTopUpTransition {
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
    use crate::state_transition::{
        StateTransitionEstimatedFeeValidation, StateTransitionHasUserFeeIncrease,
        StateTransitionLike, StateTransitionOwned, StateTransitionSingleSigned,
        StateTransitionType,
    };
    use crate::version::LATEST_PLATFORM_VERSION;
    use platform_value::{BinaryData, Identifier};

    fn make_topup() -> IdentityTopUpTransition {
        IdentityTopUpTransition::V0(IdentityTopUpTransitionV0 {
            asset_lock_proof: AssetLockProof::default(),
            identity_id: Identifier::random(),
            user_fee_increase: 1,
            signature: [0u8; 65].to_vec().into(),
        })
    }

    #[test]
    fn test_default_versioned() {
        let t = IdentityTopUpTransition::default_versioned(LATEST_PLATFORM_VERSION)
            .expect("should create default");
        match t {
            IdentityTopUpTransition::V0(_) => {}
        }
    }

    #[test]
    fn test_state_transition_like() {
        let t = make_topup();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityTopUp
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        let ids = t.modified_data_ids();
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn test_owner_id() {
        let t = make_topup();
        match &t {
            IdentityTopUpTransition::V0(v0) => {
                assert_eq!(t.owner_id(), v0.identity_id);
            }
        }
    }

    #[test]
    fn test_user_fee_increase() {
        let mut t = make_topup();
        assert_eq!(t.user_fee_increase(), 1);
        t.set_user_fee_increase(50);
        assert_eq!(t.user_fee_increase(), 50);
    }

    #[test]
    fn test_single_signed() {
        let mut t = make_topup();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![7, 8, 9]));
        assert_eq!(t.signature().as_slice(), &[7, 8, 9]);
        t.set_signature_bytes(vec![10, 11]);
        assert_eq!(t.signature().as_slice(), &[10, 11]);
    }

    #[test]
    fn test_field_types() {
        let sig = IdentityTopUpTransition::signature_property_paths();
        assert_eq!(sig.len(), 1);
        let ids = IdentityTopUpTransition::identifiers_property_paths();
        assert_eq!(ids.len(), 1);
        let bin = IdentityTopUpTransition::binary_property_paths();
        assert!(bin.is_empty());
    }

    #[test]
    fn test_estimated_fee() {
        let t = make_topup();
        let fee = t
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calc should work");
        assert!(fee > 0);
    }

    // Legacy `StateTransitionValueConvert` unknown-version tests deleted
    // in Phase D step 9 — they tested methods that no longer exist.

    #[test]
    fn test_into_from_v0() {
        let v0 = IdentityTopUpTransitionV0::default();
        let t: IdentityTopUpTransition = v0.clone().into();
        match t {
            IdentityTopUpTransition::V0(inner) => assert_eq!(inner, v0),
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

    use crate::tests::fixtures::instant_asset_lock_proof_fixture;
    use platform_value::{BinaryData, Identifier};

    // Tier 4: `instant_asset_lock_proof_fixture` produces NON-DETERMINISTIC bytes
    // (random transaction / instantLock per run), so wire-shape assertions on the
    // asset_lock_proof field stay envelope-only — the deterministic siblings
    // (identity_id, user_fee_increase, signature) get full literal assertions.
    pub(crate) fn fixture() -> IdentityTopUpTransition {
        IdentityTopUpTransition::V0(IdentityTopUpTransitionV0 {
            asset_lock_proof: instant_asset_lock_proof_fixture(None, None),
            identity_id: Identifier::new([0x44; 32]),
            user_fee_increase: 9,
            signature: BinaryData::new(vec![0xc3; 65]),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let obj = json.as_object().expect("json is an object");
        assert_eq!(obj.get("$formatVersion"), Some(&serde_json::json!("0")));
        assert_eq!(
            obj.get("identityId"),
            Some(&serde_json::json!(Identifier::new([0x44; 32])))
        );
        // `userFeeIncrease` is `u16` (UserFeeIncrease) in the source type. JSON
        // erases the size on the wire — the value-path assertion uses `9u16`.
        assert_eq!(obj.get("userFeeIncrease"), Some(&serde_json::json!(9)));
        // 65-byte signature serialized as base64 (BinaryData)
        assert_eq!(
            obj.get("signature"),
            Some(&serde_json::json!(
                "w8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8M="
            ))
        );
        let proof = obj
            .get("assetLockProof")
            .and_then(|v| v.as_object())
            .expect("assetLockProof is an object");
        assert_eq!(proof.get("$type"), Some(&serde_json::json!("instant")));
        assert_eq!(proof.get("outputIndex"), Some(&serde_json::json!(0)));
        assert!(proof.get("instantLock").is_some_and(|v| v.is_string()));
        assert!(proof.get("transaction").is_some_and(|v| v.is_string()));
        let recovered = IdentityTopUpTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let map = value.as_map().expect("value is a map");
        let get = |key: &str| {
            map.iter()
                .find(|(k, _)| k.as_text() == Some(key))
                .map(|(_, v)| v)
        };
        assert_eq!(
            get("$formatVersion"),
            Some(&platform_value::Value::Text("0".to_string()))
        );
        assert_eq!(
            get("identityId"),
            Some(&platform_value::Value::Identifier([0x44; 32]))
        );
        // `9u16`: UserFeeIncrease is `u16`; value-path preserves U16.
        assert_eq!(get("userFeeIncrease"), Some(&platform_value::Value::U16(9)));
        assert_eq!(
            get("signature"),
            Some(&platform_value::Value::Bytes(vec![0xc3; 65]))
        );
        let proof = get("assetLockProof")
            .and_then(|v| v.as_map())
            .expect("assetLockProof is a map");
        let pget = |key: &str| {
            proof
                .iter()
                .find(|(k, _)| k.as_text() == Some(key))
                .map(|(_, v)| v)
        };
        assert_eq!(
            pget("$type"),
            Some(&platform_value::Value::Text("instant".to_string()))
        );
        assert_eq!(pget("outputIndex"), Some(&platform_value::Value::U32(0)));
        assert!(pget("instantLock").is_some_and(|v| matches!(v, platform_value::Value::Bytes(_))));
        assert!(pget("transaction").is_some_and(|v| matches!(v, platform_value::Value::Bytes(_))));
        let recovered = IdentityTopUpTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
