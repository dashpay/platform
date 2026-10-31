//! Bans, unbans, suspends or unsuspends one identity on a moderated data contract (protocol
//! version 14).
//!
//! A data contract whose config declares moderation keeps a banlist and/or a suspension list
//! under its own Drive subtree. This transition, signed by the contract owner or one of the
//! moderators named in the config with a CRITICAL authentication key, edits those lists. One
//! transition carries one [`ContractUserModerationAction`].
//!
//! A banned identity, or one whose suspension has not lapsed, cannot act on the contract at the
//! document level. A lapsed suspension is deleted by the first document transition of the
//! identity that runs after it, or by an explicit unsuspend.

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
use crate::state_transition::contract_user_moderation_transition::v0::ContractUserModerationTransitionV0;
use crate::state_transition::contract_user_moderation_transition::v0::ContractUserModerationTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;
use fields::*;

use crate::data_contract::config::moderation::ContractTeamAction;
use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub use v0::ContractUserModerationAction;

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
    "dpp.state_transition_serialization_versions.contract_user_moderation_state_transition"
)]
pub enum ContractUserModerationTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(ContractUserModerationTransitionV0),
}

impl ContractUserModerationTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .contract_user_moderation_state_transition
            .default_current_version
        {
            0 => Ok(ContractUserModerationTransition::V0(
                ContractUserModerationTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ContractUserModerationTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl ContractUserModerationTransition {
    /// The team action this transition proposes or approves, by its id: for the proposal of a
    /// settled document's deletion, the id computed from its contract, its signer, its nonce,
    /// the document and the reason
    /// ([`ContractTeamAction::settled_deletion_action_id`](crate::data_contract::config::moderation::ContractTeamAction::settled_deletion_action_id)),
    /// and for an approval, the id it carries. `None` for every other action.
    pub fn team_action_id(&self) -> Option<Identifier> {
        match self {
            ContractUserModerationTransition::V0(v0) => match &v0.action {
                ContractUserModerationAction::DeleteSettledDocument {
                    document_type_name,
                    document_id,
                    reason,
                } => Some(ContractTeamAction::settled_deletion_action_id(
                    v0.data_contract_id,
                    v0.owner_id,
                    v0.identity_contract_nonce,
                    document_type_name,
                    *document_id,
                    reason,
                )),
                ContractUserModerationAction::ApproveTeamAction { action_id } => Some(*action_id),
                _ => None,
            },
        }
    }
}

impl OptionallyAssetLockProved for ContractUserModerationTransition {}

impl StateTransitionFieldTypes for ContractUserModerationTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![OWNER_ID, DATA_CONTRACT_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::data_contract::config::moderation::ContractModerationReason;
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
    use crate::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
    use crate::state_transition::{
        StateTransitionEstimatedFeeValidation, StateTransitionHasUserFeeIncrease,
        StateTransitionIdentityEstimatedFeeValidation, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use crate::version::LATEST_PLATFORM_VERSION;
    use platform_value::{BinaryData, Identifier};

    fn make() -> ContractUserModerationTransition {
        ContractUserModerationTransition::V0(ContractUserModerationTransitionV0 {
            owner_id: Identifier::random(),
            data_contract_id: Identifier::random(),
            identity_contract_nonce: 10,
            action: ContractUserModerationAction::Suspend {
                identity_id: Identifier::random(),
                until: 1_800_000_000_000,
                reason: ContractModerationReason::from_text("flooding"),
            },
            user_fee_increase: 2,
            signature_public_key_id: 0,
            signature: [0u8; 65].to_vec().into(),
        })
    }

    #[test]
    fn should_create_the_default_version() {
        let t = ContractUserModerationTransition::default_versioned(LATEST_PLATFORM_VERSION)
            .expect("should create default");
        match t {
            ContractUserModerationTransition::V0(_) => {}
        }
    }

    #[test]
    fn should_round_trip_through_bytes_for_every_action() {
        let target = Identifier::random();
        for action in [
            ContractUserModerationAction::Ban {
                identity_id: target,
                reason: ContractModerationReason {
                    code: Some(u16::MAX),
                    text: "spam".to_string(),
                    documents: vec![],
                    reason_document_id: None,
                },
            },
            ContractUserModerationAction::Unban {
                identity_id: target,
            },
            ContractUserModerationAction::Suspend {
                identity_id: target,
                until: 5,
                reason: ContractModerationReason::default(),
            },
            ContractUserModerationAction::Unsuspend {
                identity_id: target,
            },
            ContractUserModerationAction::DeleteSettledDocument {
                document_type_name: "post".to_string(),
                document_id: target,
                reason: ContractModerationReason::from_text("doxxing"),
            },
            ContractUserModerationAction::ApproveTeamAction { action_id: target },
        ] {
            let mut t = make();
            t.set_action(action);
            let bytes = t.serialize_to_bytes().expect("should serialize");
            let restored =
                ContractUserModerationTransition::deserialize_from_bytes_untrusted(&bytes)
                    .expect("should deserialize");
            assert_eq!(t, restored);
        }
    }

    #[test]
    fn should_report_its_type_and_identifiers() {
        let t = make();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::ContractUserModeration
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.data_contract_id()]);
        assert_eq!(t.unique_identifiers().len(), 1);
        let ContractUserModerationTransition::V0(v0) = &t;
        assert_eq!(t.owner_id(), v0.owner_id);
        assert_eq!(t.action().identity_id(), t.target_identity_id());
    }

    #[test]
    fn should_expose_the_user_fee_increase_and_signature() {
        let mut t = make();
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
        let mut t = make();
        assert_eq!(t.identity_contract_nonce(), 10);
        t.set_identity_contract_nonce(20);
        assert_eq!(t.identity_contract_nonce(), 20);
        let contract_id = Identifier::random();
        t.set_data_contract_id(contract_id);
        assert_eq!(t.data_contract_id(), contract_id);
        let owner_id = Identifier::random();
        t.set_owner_id(owner_id);
        assert_eq!(t.owner_id(), owner_id);
    }

    #[test]
    fn should_list_its_field_types() {
        assert_eq!(
            ContractUserModerationTransition::signature_property_paths().len(),
            2
        );
        assert_eq!(
            ContractUserModerationTransition::identifiers_property_paths().len(),
            2
        );
        assert_eq!(
            ContractUserModerationTransition::binary_property_paths().len(),
            1
        );
    }

    #[test]
    fn should_require_the_contract_update_minimum_fee() {
        let t = make();
        let fee = t
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calc should work");
        assert_eq!(
            fee,
            LATEST_PLATFORM_VERSION
                .fee_version
                .state_transition_min_fees
                .contract_update
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

    use crate::data_contract::config::moderation::ContractModerationReason;
    use crate::serialization::{JsonConvertible, Signable, ValueConvertible};
    use crate::state_transition::contract_user_moderation_transition::ContractUserModerationAction;
    use platform_value::{platform_value, BinaryData, Identifier, Value};
    use serde_json::json;
    use std::collections::BTreeMap;

    pub(crate) fn fixture() -> ContractUserModerationTransition {
        ContractUserModerationTransition::V0(ContractUserModerationTransitionV0 {
            owner_id: Identifier::new([0x55; 32]),
            data_contract_id: Identifier::new([0x66; 32]),
            identity_contract_nonce: 17,
            action: ContractUserModerationAction::Suspend {
                identity_id: Identifier::new([0x77; 32]),
                until: 1_800_000_000_000,
                reason: ContractModerationReason {
                    code: Some(12),
                    text: "flooding".to_string(),
                    documents: vec![],
                    reason_document_id: None,
                },
            },
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
                "ownerId": Identifier::new([0x55; 32]),
                "dataContractId": Identifier::new([0x66; 32]),
                "identityContractNonce": 17,
                "action": {
                    "$type": "suspend",
                    "identityId": Identifier::new([0x77; 32]),
                    "until": 1_800_000_000_000u64,
                    "reason": {"code": 12, "text": "flooding"},
                },
                "userFeeIncrease": 4,
                "signaturePublicKeyId": 6,
                "signature": "1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NTU1NQ=",
            })
        );
        let recovered = ContractUserModerationTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    /// A field change with a value of every kind the canonical form settles
    fn field_change() -> ContractUserModerationTransition {
        let ContractUserModerationTransition::V0(base) = fixture();
        ContractUserModerationTransition::V0(ContractUserModerationTransitionV0 {
            action: ContractUserModerationAction::ChangeDocumentFields {
                document_type_name: "report".to_string(),
                document_id: Identifier::new([0x77; 32]),
                fields: BTreeMap::from([
                    ("status".to_string(), Value::U64(2)),
                    ("offset".to_string(), Value::I64(-3)),
                    ("weight".to_string(), Value::Float(1.5)),
                    ("reviewer".to_string(), Value::Identifier([0x88; 32])),
                    ("attachment".to_string(), Value::Bytes(vec![1, 2, 3])),
                    ("resolution".to_string(), Value::Null),
                ]),
                reason: ContractModerationReason::from_text("handled"),
            },
            ..base
        })
    }

    #[test]
    fn should_read_back_a_field_change_that_signs_the_same_bytes() {
        let transition = field_change();
        let signed = transition.signable_bytes().expect("signable");

        // The object form as JavaScript gives it back: a bigint read as the signed width, an
        // identifier as plain bytes. Read back, the change signs the bytes it was built with.
        let mut object = transition.to_object().expect("to_object");
        object
            .set_value_at_full_path("action.fields.status", Value::I64(2))
            .expect("the status is in the object");
        object
            .set_value_at_full_path("action.fields.reviewer", Value::Bytes(vec![0x88; 32]))
            .expect("the reviewer is in the object");
        let read_back = ContractUserModerationTransition::from_object(object).expect("from_object");
        assert_eq!(read_back, transition);
        assert_eq!(read_back.signable_bytes().expect("signable"), signed);

        // Through JSON, which writes a binary value as text, a change of numbers and removals
        // reads back the same.
        let ContractUserModerationTransition::V0(mut numeric) = transition;
        if let ContractUserModerationAction::ChangeDocumentFields { fields, .. } =
            &mut numeric.action
        {
            fields.retain(|_, value| value.as_bytes_slice().is_err());
        }
        let numeric = ContractUserModerationTransition::V0(numeric);
        let read_back =
            ContractUserModerationTransition::from_json(numeric.to_json().expect("to_json"))
                .expect("from_json");
        assert_eq!(
            read_back.signable_bytes().expect("signable"),
            numeric.signable_bytes().expect("signable")
        );
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
                "ownerId": Identifier::new([0x55; 32]),
                "dataContractId": Identifier::new([0x66; 32]),
                "identityContractNonce": 17u64,
                "action": {
                    "$type": "suspend",
                    "identityId": Identifier::new([0x77; 32]),
                    "until": 1_800_000_000_000u64,
                    "reason": {"code": 12u16, "text": "flooding"},
                },
                "userFeeIncrease": 4u16,
                "signaturePublicKeyId": 6u32,
                "signature": BinaryData::new(vec![0xd4; 65]),
            })
        );
        let recovered = ContractUserModerationTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
