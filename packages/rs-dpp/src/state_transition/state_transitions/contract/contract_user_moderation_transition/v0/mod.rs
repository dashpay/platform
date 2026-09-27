mod identity_signed;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::data_contract::config::moderation::ContractModerationReason;
use crate::identity::{KeyID, TimestampMillis};
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use crate::ProtocolError;
use std::fmt;

/// What the moderator does on the contract: to one identity, or to one document.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$type", rename_all = "camelCase")
)]
pub enum ContractUserModerationAction {
    /// Puts the identity on the banlist. Removes a suspension it may carry.
    Ban {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "identityId"))]
        identity_id: Identifier,
        /// Why, stored with the banlist entry.
        reason: ContractModerationReason,
    },
    /// Takes the identity off the banlist.
    Unban {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "identityId"))]
        identity_id: Identifier,
    },
    /// Puts the identity on the suspension list until `until` (block time, milliseconds),
    /// replacing a suspension it already carries.
    Suspend {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "identityId"))]
        identity_id: Identifier,
        until: TimestampMillis,
        /// Why, stored with the suspension list entry.
        reason: ContractModerationReason,
    },
    /// Takes the identity off the suspension list, lapsed or not.
    Unsuspend {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "identityId"))]
        identity_id: Identifier,
    },
    /// Adds a warning, with the block time and `reason`, to the identity's entry on the
    /// warning list, which bars it from nothing. Refused once the entry holds
    /// `SystemLimits::max_contract_warnings_per_identity` warnings.
    Warn {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "identityId"))]
        identity_id: Identifier,
        /// Why, stored with the warning.
        reason: ContractModerationReason,
    },
    /// Takes the identity off the warning list: every warning it carries goes.
    ClearWarnings {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "identityId"))]
        identity_id: Identifier,
    },
    /// Deletes a document of a document type that sets `canBeDeletedByModerators`, whoever
    /// owns it, except the contract owner and the moderators. The document's owner gets no
    /// storage refund, and a `ContractDocumentRemoval` stays under the contract.
    DeleteDocument {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "documentTypeName"))]
        document_type_name: String,
        #[cfg_attr(feature = "serde-conversion", serde(rename = "documentId"))]
        document_id: Identifier,
        /// Why, stored with the removal record.
        reason: ContractModerationReason,
    },
    /// Brings back a document a moderator deleted, as it was: the document serialized under
    /// its document type (`Document::serialize`), which must hash to what its removal record
    /// holds, within `SystemLimits::contract_document_restore_window_ms` of the removal. The
    /// document's id, owner and content are all inside the bytes. The record stays, marked
    /// restored; the signer pays for the document's storage, whose refund stays the owner's.
    RestoreDocument {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "documentTypeName"))]
        document_type_name: String,
        /// The document as it was serialized when it was removed.
        document: BinaryData,
    },
}

impl Default for ContractUserModerationAction {
    fn default() -> Self {
        ContractUserModerationAction::Ban {
            identity_id: Identifier::default(),
            reason: ContractModerationReason::default(),
        }
    }
}

impl ContractUserModerationAction {
    /// The identity the action targets. `None` for a document deletion: it names a document,
    /// and whose it is is only known once the document is read.
    pub fn identity_id(&self) -> Option<Identifier> {
        match self {
            ContractUserModerationAction::Ban { identity_id, .. }
            | ContractUserModerationAction::Unban { identity_id }
            | ContractUserModerationAction::Suspend { identity_id, .. }
            | ContractUserModerationAction::Unsuspend { identity_id }
            | ContractUserModerationAction::Warn { identity_id, .. }
            | ContractUserModerationAction::ClearWarnings { identity_id } => Some(*identity_id),
            ContractUserModerationAction::DeleteDocument { .. }
            | ContractUserModerationAction::RestoreDocument { .. } => None,
        }
    }

    /// The document a deletion targets, as its document type name and its id. `None` for an
    /// action on an identity, and for a restore, which carries the document itself: its id is
    /// only known once the bytes are decoded under the document type.
    pub fn document(&self) -> Option<(&str, Identifier)> {
        match self {
            ContractUserModerationAction::DeleteDocument {
                document_type_name,
                document_id,
                ..
            } => Some((document_type_name.as_str(), *document_id)),
            _ => None,
        }
    }

    /// The document a restore brings back, as its document type name and its serialized
    /// bytes. `None` for every other action.
    pub fn restored_document(&self) -> Option<(&str, &[u8])> {
        match self {
            ContractUserModerationAction::RestoreDocument {
                document_type_name,
                document,
            } => Some((document_type_name.as_str(), document.as_slice())),
            _ => None,
        }
    }

    /// The document type name a deletion or a restore names, `None` for an action on an
    /// identity.
    pub fn document_type_name(&self) -> Option<&str> {
        match self {
            ContractUserModerationAction::DeleteDocument {
                document_type_name, ..
            }
            | ContractUserModerationAction::RestoreDocument {
                document_type_name, ..
            } => Some(document_type_name.as_str()),
            _ => None,
        }
    }

    /// The suspension end for a suspend, `None` otherwise.
    pub fn until(&self) -> Option<TimestampMillis> {
        match self {
            ContractUserModerationAction::Suspend { until, .. } => Some(*until),
            _ => None,
        }
    }

    /// The reason a ban, a suspend, a warn or a document deletion carries, `None` for an
    /// action that takes an identity off a list.
    pub fn reason(&self) -> Option<&ContractModerationReason> {
        match self {
            ContractUserModerationAction::Ban { reason, .. }
            | ContractUserModerationAction::Suspend { reason, .. }
            | ContractUserModerationAction::Warn { reason, .. }
            | ContractUserModerationAction::DeleteDocument { reason, .. } => Some(reason),
            ContractUserModerationAction::Unban { .. }
            | ContractUserModerationAction::Unsuspend { .. }
            | ContractUserModerationAction::ClearWarnings { .. }
            | ContractUserModerationAction::RestoreDocument { .. } => None,
        }
    }

    /// The action's name on the wire.
    pub fn name(&self) -> &'static str {
        match self {
            ContractUserModerationAction::Ban { .. } => "ban",
            ContractUserModerationAction::Unban { .. } => "unban",
            ContractUserModerationAction::Suspend { .. } => "suspend",
            ContractUserModerationAction::Unsuspend { .. } => "unsuspend",
            ContractUserModerationAction::Warn { .. } => "warn",
            ContractUserModerationAction::ClearWarnings { .. } => "clearWarnings",
            ContractUserModerationAction::DeleteDocument { .. } => "deleteDocument",
            ContractUserModerationAction::RestoreDocument { .. } => "restoreDocument",
        }
    }
}

impl fmt::Display for ContractUserModerationAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractUserModerationAction::Suspend {
                identity_id, until, ..
            } => {
                write!(f, "suspend {} until {}", identity_id, until)
            }
            ContractUserModerationAction::DeleteDocument {
                document_type_name,
                document_id,
                ..
            } => {
                write!(f, "delete {} document {}", document_type_name, document_id)
            }
            ContractUserModerationAction::RestoreDocument {
                document_type_name,
                document,
            } => {
                write!(
                    f,
                    "restore {} document of {} bytes",
                    document_type_name,
                    document.len()
                )
            }
            ContractUserModerationAction::Ban { identity_id, .. }
            | ContractUserModerationAction::Unban { identity_id }
            | ContractUserModerationAction::Unsuspend { identity_id }
            | ContractUserModerationAction::Warn { identity_id, .. }
            | ContractUserModerationAction::ClearWarnings { identity_id } => {
                write!(f, "{} {}", self.name(), identity_id)
            }
        }
    }
}

// `until` is a u64, but basic structure validation refuses one past
// `SystemLimits::max_contract_suspension_until` (2^53 - 1), so it is exact in JSON. The
// reason holds a u16 and a string.
#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ContractUserModerationAction {}

/// Edits the banlist, the suspension list or the warning list of a moderated data contract,
/// or deletes or restores a document of one of its document types that moderators may
/// delete. Signed by the
/// contract owner or a moderator named in the contract's config, with a CRITICAL
/// authentication key, under the signer's contract-scoped nonce.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Encode, Decode, PlatformSignable, Debug, Clone, PartialEq, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct ContractUserModerationTransitionV0 {
    /// The moderator: the identity that signs.
    pub owner_id: Identifier,

    /// The moderated contract.
    pub data_contract_id: Identifier,

    /// The signer's nonce for this contract, to prevent replay attacks.
    pub identity_contract_nonce: IdentityNonce,

    /// What is done, to whom.
    pub action: ContractUserModerationAction,

    /// The fee multiplier
    pub user_fee_increase: UserFeeIncrease,

    /// The ID of the public key used to sign the State Transition
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    /// Cryptographic signature of the State Transition
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::serialization::Signable;
    use crate::state_transition::{
        StateTransition, StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned,
        StateTransitionLike, StateTransitionOwned, StateTransitionSingleSigned,
        StateTransitionType,
    };

    fn make_v0() -> ContractUserModerationTransitionV0 {
        ContractUserModerationTransitionV0 {
            owner_id: Identifier::random(),
            data_contract_id: Identifier::random(),
            identity_contract_nonce: 3,
            action: ContractUserModerationAction::Ban {
                identity_id: Identifier::random(),
                reason: ContractModerationReason::from_text("spam"),
            },
            user_fee_increase: 1,
            signature_public_key_id: 2,
            signature: BinaryData::new(vec![9; 65]),
        }
    }

    #[test]
    fn should_exclude_the_signature_from_the_signable_bytes() {
        let a = make_v0();
        let mut b = a.clone();
        b.signature = BinaryData::new(vec![1; 65]);
        b.signature_public_key_id = 7;
        assert_eq!(
            a.signable_bytes().expect("signable"),
            b.signable_bytes().expect("signable")
        );
    }

    #[test]
    fn should_describe_itself() {
        let mut t = make_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::ContractUserModeration
        );
        assert_eq!(t.modified_data_ids(), vec![t.data_contract_id]);
        assert_eq!(t.owner_id(), t.owner_id);
        assert_eq!(t.signature_public_key_id(), 2);
        t.set_signature_public_key_id(4);
        assert_eq!(t.signature_public_key_id(), 4);
        assert_eq!(t.user_fee_increase(), 1);
        t.set_user_fee_increase(9);
        assert_eq!(t.user_fee_increase(), 9);
        assert_eq!(t.signature().len(), 65);
        assert_eq!(
            t.security_level_requirement(crate::identity::Purpose::AUTHENTICATION),
            vec![crate::identity::SecurityLevel::CRITICAL]
        );
        let outer: StateTransition = t.into();
        assert!(matches!(outer, StateTransition::ContractUserModeration(_)));
    }

    #[test]
    fn should_name_the_action_target() {
        let target = Identifier::random();
        let reason = ContractModerationReason {
            code: Some(4),
            text: "flooding".to_string(),
            documents: vec![],
            reason_document_id: None,
        };
        let action = ContractUserModerationAction::Suspend {
            identity_id: target,
            until: 12,
            reason: reason.clone(),
        };
        assert_eq!(action.identity_id(), Some(target));
        assert_eq!(action.document(), None);
        assert_eq!(action.until(), Some(12));
        assert_eq!(action.reason(), Some(&reason));
        assert_eq!(action.name(), "suspend");
        let unban = ContractUserModerationAction::Unban {
            identity_id: target,
        };
        assert_eq!(unban.until(), None);
        assert_eq!(unban.reason(), None);
    }

    #[test]
    fn should_name_the_target_of_a_warning_and_of_its_clearing() {
        let target = Identifier::random();
        let reason = ContractModerationReason::from_text("first strike");
        let warn = ContractUserModerationAction::Warn {
            identity_id: target,
            reason: reason.clone(),
        };
        assert_eq!(warn.identity_id(), Some(target));
        assert_eq!(warn.document(), None);
        assert_eq!(warn.until(), None);
        assert_eq!(warn.reason(), Some(&reason));
        assert_eq!(warn.name(), "warn");
        assert_eq!(warn.to_string(), format!("warn {}", target));
        let clear = ContractUserModerationAction::ClearWarnings {
            identity_id: target,
        };
        assert_eq!(clear.identity_id(), Some(target));
        assert_eq!(clear.reason(), None);
        assert_eq!(clear.name(), "clearWarnings");
    }

    #[cfg(feature = "json-conversion")]
    #[test]
    fn should_tag_a_warning_on_the_wire() {
        let action = ContractUserModerationAction::Warn {
            identity_id: Identifier::from([7; 32]),
            reason: ContractModerationReason::from_text("spam"),
        };
        let json = serde_json::to_value(&action).expect("to json");
        assert_eq!(json["$type"], "warn");
        let back: ContractUserModerationAction = serde_json::from_value(json).expect("from json");
        assert_eq!(back, action);
        let clear = ContractUserModerationAction::ClearWarnings {
            identity_id: Identifier::from([7; 32]),
        };
        let json = serde_json::to_value(&clear).expect("to json");
        assert_eq!(json["$type"], "clearWarnings");
        assert!(json.get("reason").is_none());
    }

    #[test]
    fn should_name_the_document_of_a_deletion() {
        let document_id = Identifier::random();
        let reason = ContractModerationReason {
            code: Some(2),
            text: "spam".to_string(),
            documents: vec![],
            reason_document_id: None,
        };
        let action = ContractUserModerationAction::DeleteDocument {
            document_type_name: "post".to_string(),
            document_id,
            reason: reason.clone(),
        };
        // A deletion names a document: whose it is is only known once it is read.
        assert_eq!(action.identity_id(), None);
        assert_eq!(action.document(), Some(("post", document_id)));
        assert_eq!(action.until(), None);
        assert_eq!(action.reason(), Some(&reason));
        assert_eq!(action.name(), "deleteDocument");
        assert_eq!(
            action.to_string(),
            format!("delete post document {}", document_id)
        );
    }

    #[test]
    fn should_name_the_document_of_a_restore_by_its_bytes() {
        let action = ContractUserModerationAction::RestoreDocument {
            document_type_name: "post".to_string(),
            document: BinaryData::new(vec![7; 70]),
        };
        // A restore carries the document: whose it is, and which id, is inside the bytes.
        assert_eq!(action.identity_id(), None);
        assert_eq!(action.document(), None);
        assert_eq!(action.restored_document(), Some(("post", &[7u8; 70][..])));
        assert_eq!(action.document_type_name(), Some("post"));
        assert_eq!(action.until(), None);
        assert_eq!(action.reason(), None);
        assert_eq!(action.name(), "restoreDocument");
        assert_eq!(action.to_string(), "restore post document of 70 bytes");
    }

    #[cfg(feature = "json-conversion")]
    #[test]
    fn should_tag_a_restore_on_the_wire() {
        let action = ContractUserModerationAction::RestoreDocument {
            document_type_name: "post".to_string(),
            document: BinaryData::new(vec![7; 70]),
        };
        let json = serde_json::to_value(&action).expect("to json");
        assert_eq!(json["$type"], "restoreDocument");
        assert_eq!(json["documentTypeName"], "post");
        assert!(json["document"].is_string(), "the bytes travel as a string");
        let back: ContractUserModerationAction = serde_json::from_value(json).expect("from json");
        assert_eq!(back, action);
    }

    #[cfg(feature = "json-conversion")]
    #[test]
    fn should_tag_a_deletion_on_the_wire() {
        let action = ContractUserModerationAction::DeleteDocument {
            document_type_name: "post".to_string(),
            document_id: Identifier::from([7; 32]),
            reason: ContractModerationReason::default(),
        };
        let json = serde_json::to_value(&action).expect("to json");
        assert_eq!(json["$type"], "deleteDocument");
        assert_eq!(json["documentTypeName"], "post");
        assert!(json.get("documentId").is_some());
        let back: ContractUserModerationAction = serde_json::from_value(json).expect("from json");
        assert_eq!(back, action);
    }

    #[test]
    fn should_sign_the_reason() {
        let a = make_v0();
        let mut b = a.clone();
        b.action = ContractUserModerationAction::Ban {
            identity_id: a.action.identity_id().expect("a ban names an identity"),
            reason: ContractModerationReason::from_text("something else"),
        };
        assert_ne!(
            a.signable_bytes().expect("signable"),
            b.signable_bytes().expect("signable")
        );
    }
}
