//! Raises the limits of one of the identity's own authentication keys (protocol version 14).
//!
//! A key registered with a `total_budget` or an `expires_at` cannot be edited in place by an
//! [`IdentityUpdateTransition`](crate::state_transition::identity_update_transition), which only
//! adds and disables keys. This transition raises the budget (the key's stored `total_budget` and
//! the remaining budget Drive keeps for it grow by the same amount) and moves the expiry later.
//! It only ever loosens: a limit can not be lowered, and a limit the key does not have can not be
//! added; tightening is what disabling is for.
//!
//! Signed by a MASTER key, or by a CRITICAL authentication key that carries no limits itself.

pub mod accessors;
pub mod fields;
mod identity_signed;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
pub mod v0;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;
use crate::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;
use fields::*;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};
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
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.identity_key_limits_update_state_transition"
)]
pub enum IdentityKeyLimitsUpdateTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityKeyLimitsUpdateTransitionV0),
}

impl IdentityKeyLimitsUpdateTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .identity_key_limits_update_state_transition
            .default_current_version
        {
            0 => Ok(IdentityKeyLimitsUpdateTransition::V0(
                IdentityKeyLimitsUpdateTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityKeyLimitsUpdateTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl OptionallyAssetLockProved for IdentityKeyLimitsUpdateTransition {}

impl StateTransitionFieldTypes for IdentityKeyLimitsUpdateTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
    use crate::state_transition::identity_key_limits_update_transition::accessors::IdentityKeyLimitsUpdateTransitionAccessorsV0;
    use crate::state_transition::{
        StateTransitionEstimatedFeeValidation, StateTransitionHasUserFeeIncrease,
        StateTransitionIdentityEstimatedFeeValidation, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use crate::version::LATEST_PLATFORM_VERSION;
    use platform_value::{BinaryData, Identifier};

    fn make_update() -> IdentityKeyLimitsUpdateTransition {
        IdentityKeyLimitsUpdateTransition::V0(IdentityKeyLimitsUpdateTransitionV0 {
            identity_id: Identifier::random(),
            nonce: 10,
            key_id: 7,
            total_budget: Some(500_000_000),
            expires_at: None,
            user_fee_increase: 2,
            signature_public_key_id: 0,
            signature: [0u8; 65].to_vec().into(),
        })
    }

    #[test]
    fn should_create_the_default_version() {
        let t = IdentityKeyLimitsUpdateTransition::default_versioned(LATEST_PLATFORM_VERSION)
            .expect("should create default");
        match t {
            IdentityKeyLimitsUpdateTransition::V0(_) => {}
        }
    }

    #[test]
    fn should_round_trip_through_bytes() {
        let t = make_update();
        let bytes = t.serialize_to_bytes().expect("should serialize");
        let restored = IdentityKeyLimitsUpdateTransition::deserialize_from_bytes_untrusted(&bytes)
            .expect("should deserialize");
        assert_eq!(t, restored);
    }

    #[test]
    fn should_report_its_type_and_identifiers() {
        let t = make_update();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityKeyLimitsUpdate
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.identity_id()]);
        assert_eq!(t.unique_identifiers().len(), 1);
        assert_eq!(t.owner_id(), t.identity_id());
    }

    #[test]
    fn should_expose_the_user_fee_increase_and_signature() {
        let mut t = make_update();
        assert_eq!(t.user_fee_increase(), 2);
        t.set_user_fee_increase(50);
        assert_eq!(t.user_fee_increase(), 50);
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2]));
        assert_eq!(t.signature().as_slice(), &[1, 2]);
        t.set_signature_bytes(vec![3, 4]);
        assert_eq!(t.signature().as_slice(), &[3, 4]);
    }

    #[test]
    fn should_expose_the_accessors() {
        let mut t = make_update();
        assert_eq!(t.nonce(), 10);
        t.set_nonce(20);
        assert_eq!(t.nonce(), 20);
        assert_eq!(t.key_id(), 7);
        t.set_key_id(9);
        assert_eq!(t.key_id(), 9);
        assert_eq!(t.total_budget(), Some(500_000_000));
        t.set_total_budget(None);
        assert_eq!(t.total_budget(), None);
        assert_eq!(t.expires_at(), None);
        t.set_expires_at(Some(1_700_000_000_000));
        assert_eq!(t.expires_at(), Some(1_700_000_000_000));
    }

    #[test]
    fn should_list_its_field_types() {
        assert_eq!(
            IdentityKeyLimitsUpdateTransition::signature_property_paths().len(),
            2
        );
        assert_eq!(
            IdentityKeyLimitsUpdateTransition::identifiers_property_paths().len(),
            1
        );
        assert_eq!(
            IdentityKeyLimitsUpdateTransition::binary_property_paths().len(),
            1
        );
    }

    #[test]
    fn should_require_the_identity_update_minimum_fee() {
        let t = make_update();
        let fee = t
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calc should work");
        assert_eq!(
            fee,
            LATEST_PLATFORM_VERSION
                .fee_version
                .state_transition_min_fees
                .identity_update
        );
        assert!(t
            .validate_estimated_fee(fee, LATEST_PLATFORM_VERSION)
            .expect("validation should work")
            .is_valid());
        assert!(!t
            .validate_estimated_fee(fee - 1, LATEST_PLATFORM_VERSION)
            .expect("validation should work")
            .is_valid());
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

    pub(crate) fn fixture() -> IdentityKeyLimitsUpdateTransition {
        IdentityKeyLimitsUpdateTransition::V0(IdentityKeyLimitsUpdateTransitionV0 {
            identity_id: Identifier::new([0x55; 32]),
            nonce: 17,
            key_id: 5,
            total_budget: Some(1_000_000_000),
            expires_at: Some(1_800_000_000_000),
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
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "identityId": Identifier::new([0x55; 32]),
                "nonce": 17,
                "keyId": 5,
                "totalBudget": 1_000_000_000u64,
                "expiresAt": 1_800_000_000_000u64,
                "userFeeIncrease": 4,
                "signaturePublicKeyId": 6,
                "signature": "1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NQ=",
            })
        );
        let recovered = IdentityKeyLimitsUpdateTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "identityId": Identifier::new([0x55; 32]),
                "nonce": 17u64,
                "keyId": 5u32,
                "totalBudget": 1_000_000_000u64,
                "expiresAt": 1_800_000_000_000u64,
                "userFeeIncrease": 4u16,
                "signaturePublicKeyId": 6u32,
                "signature": BinaryData::new(vec![0xd4; 65]),
            })
        );
        let recovered = IdentityKeyLimitsUpdateTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
