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
use crate::state_transition::identity_credit_transfer_transition::fields::property_names::RECIPIENT_ID;
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0Signable;
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

pub type IdentityCreditTransferTransitionLatest = IdentityCreditTransferTransitionV0;

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
    "dpp.state_transition_serialization_versions.identity_credit_transfer_state_transition"
)]
pub enum IdentityCreditTransferTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityCreditTransferTransitionV0),
}

impl IdentityCreditTransferTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityCreditTransferTransition::V0(
                IdentityCreditTransferTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreditTransferTransitionV0::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl OptionallyAssetLockProved for IdentityCreditTransferTransition {}

impl StateTransitionFieldTypes for IdentityCreditTransferTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID, RECIPIENT_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use crate::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
    use crate::state_transition::{
        StateTransitionEstimatedFeeValidation, StateTransitionHasUserFeeIncrease,
        StateTransitionIdentityEstimatedFeeValidation, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use crate::version::LATEST_PLATFORM_VERSION;
    use platform_value::{BinaryData, Identifier};

    fn make_transfer() -> IdentityCreditTransferTransition {
        IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::random(),
            recipient_id: Identifier::random(),
            amount: 500_000,
            nonce: 7,
            user_fee_increase: 3,
            signature_public_key_id: 2,
            signature: [0u8; 65].to_vec().into(),
        })
    }

    #[test]
    fn test_default_versioned() {
        let transition =
            IdentityCreditTransferTransition::default_versioned(LATEST_PLATFORM_VERSION)
                .expect("should create default");
        match transition {
            IdentityCreditTransferTransition::V0(_) => {}
        }
    }

    #[test]
    fn test_serialization_roundtrip() {
        let transition = make_transfer();
        let bytes = transition.serialize_to_bytes().expect("should serialize");
        let restored = IdentityCreditTransferTransition::deserialize_from_bytes(&bytes)
            .expect("should deserialize");
        assert_eq!(transition, restored);
    }

    #[test]
    fn test_state_transition_like() {
        let transition = make_transfer();
        assert_eq!(
            transition.state_transition_type(),
            StateTransitionType::IdentityCreditTransfer
        );
        assert_eq!(transition.state_transition_protocol_version(), 0);
        let ids = transition.modified_data_ids();
        assert_eq!(ids.len(), 2);
        let unique = transition.unique_identifiers();
        assert_eq!(unique.len(), 1);
        assert!(!unique[0].is_empty());
    }

    #[test]
    fn test_owner_id() {
        let transition = make_transfer();
        match &transition {
            IdentityCreditTransferTransition::V0(v0) => {
                assert_eq!(transition.owner_id(), v0.identity_id);
            }
        }
    }

    #[test]
    fn test_user_fee_increase() {
        let mut transition = make_transfer();
        assert_eq!(transition.user_fee_increase(), 3);
        transition.set_user_fee_increase(99);
        assert_eq!(transition.user_fee_increase(), 99);
    }

    #[test]
    fn test_single_signed() {
        let mut transition = make_transfer();
        assert_eq!(transition.signature().len(), 65);
        transition.set_signature(BinaryData::new(vec![1, 2, 3]));
        assert_eq!(transition.signature().as_slice(), &[1, 2, 3]);
        transition.set_signature_bytes(vec![4, 5]);
        assert_eq!(transition.signature().as_slice(), &[4, 5]);
    }

    #[test]
    fn test_accessors() {
        let mut transition = make_transfer();
        assert_eq!(transition.amount(), 500_000);
        transition.set_amount(1_000_000);
        assert_eq!(transition.amount(), 1_000_000);
        assert_eq!(transition.nonce(), 7);
        transition.set_nonce(42);
        assert_eq!(transition.nonce(), 42);
        let old_recipient = transition.recipient_id();
        let new_recipient = Identifier::random();
        transition.set_recipient_id(new_recipient);
        assert_eq!(transition.recipient_id(), new_recipient);
        assert_ne!(transition.recipient_id(), old_recipient);
    }

    #[test]
    fn test_field_types() {
        let sig_paths = IdentityCreditTransferTransition::signature_property_paths();
        assert_eq!(sig_paths.len(), 1);
        let id_paths = IdentityCreditTransferTransition::identifiers_property_paths();
        assert_eq!(id_paths.len(), 2);
        let bin_paths = IdentityCreditTransferTransition::binary_property_paths();
        assert!(bin_paths.is_empty());
    }

    // Legacy `StateTransitionValueConvert` round-trip and
    // unknown-version tests deleted in Phase D step 9. The canonical
    // `JsonConvertible` / `ValueConvertible` round-trip is exercised on
    // the outer enum derive (see `json_convertible_tests` below) — these
    // tested methods that no longer exist.

    #[test]
    fn test_estimated_fee_validation_sufficient() {
        let transition = make_transfer();
        let fee = transition
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calculation should work");
        assert!(fee > 0);
        let result = transition
            .validate_estimated_fee(fee + transition.amount() + 1000, LATEST_PLATFORM_VERSION)
            .expect("validation should succeed");
        assert!(result.is_valid());
    }

    #[test]
    fn test_estimated_fee_validation_insufficient() {
        let transition = make_transfer();
        let result = transition
            .validate_estimated_fee(0, LATEST_PLATFORM_VERSION)
            .expect("validation should succeed");
        assert!(!result.is_valid());
    }

    #[test]
    fn test_into_from_v0() {
        let v0 = IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::random(),
            recipient_id: Identifier::random(),
            amount: 42,
            nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: vec![].into(),
        };
        let transition: IdentityCreditTransferTransition = v0.clone().into();
        match transition {
            IdentityCreditTransferTransition::V0(inner) => assert_eq!(inner, v0),
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

    pub(crate) fn fixture() -> IdentityCreditTransferTransition {
        IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::new([0x11; 32]),
            recipient_id: Identifier::new([0x22; 32]),
            amount: 1_234_567,
            nonce: 42,
            user_fee_increase: 7,
            signature_public_key_id: 3,
            signature: BinaryData::new(vec![0xa1; 65]),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields whose wire encoding loses size info (JSON has only one
        // number type): `nonce` (u64), `userFeeIncrease` (u16),
        // `signaturePublicKeyId` (u32). The value-path assertion below uses the
        // explicit suffixes to lock in the typed variants.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "identityId": Identifier::new([0x11; 32]),
                "recipientId": Identifier::new([0x22; 32]),
                "amount": 1_234_567,
                "nonce": 42,
                "userFeeIncrease": 7,
                "signaturePublicKeyId": 3,
                "signature": "oaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaE=",
            })
        );
        let recovered = IdentityCreditTransferTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Explicit suffixes lock in sized variants: `nonce` u64, `userFeeIncrease`
        // u16, `signaturePublicKeyId` u32 (KeyID alias).
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "identityId": Identifier::new([0x11; 32]),
                "recipientId": Identifier::new([0x22; 32]),
                "amount": 1_234_567u64,
                "nonce": 42u64,
                "userFeeIncrease": 7u16,
                "signaturePublicKeyId": 3u32,
                "signature": BinaryData::new(vec![0xa1; 65]),
            })
        );
        let recovered = IdentityCreditTransferTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
