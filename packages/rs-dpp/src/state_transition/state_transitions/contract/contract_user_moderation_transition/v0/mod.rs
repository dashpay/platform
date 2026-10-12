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
use platform_value::{BinaryData, Value};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::data_contract::config::moderation::ContractModerationReason;
use crate::identity::{KeyID, TimestampMillis};
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use crate::ProtocolError;
use std::collections::BTreeMap;
use std::fmt;

/// What the moderator does on the contract: to one identity, or to one document.
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
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
    /// Deletes a document of a document type that sets `moderatorAbilities.delete`, whoever
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
    /// Sets or removes fields of a document that only moderators write, the ones its document
    /// type lists under `moderatorAbilities.changeFields`, whoever owns it. Every other
    /// property, `$updatedAt` among them, stays as it was; `$revision` goes up by one, so a
    /// replace its owner built on the earlier revision is refused. The signer pays for the
    /// bytes the change adds, and a refund of the document's storage stays the owner's.
    ChangeDocumentFields {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "documentTypeName"))]
        document_type_name: String,
        #[cfg_attr(feature = "serde-conversion", serde(rename = "documentId"))]
        document_id: Identifier,
        /// Each field's new value, by its top-level property name; `null` removes the field.
        /// Read from an object or JSON in its canonical form
        /// ([`ContractUserModerationAction::canonical_field_value`]), so a transition built by
        /// a client and read back signs the same bytes.
        #[cfg_attr(
            feature = "serde-conversion",
            serde(deserialize_with = "deserialize_canonical_fields")
        )]
        fields: BTreeMap<String, Value>,
        /// Why, checked against a seated team's proposal like every other reason.
        reason: ContractModerationReason,
    },
    /// Proposes the deletion of a settled document: one of a document type that says who must
    /// approve it (`moderatorAbilities.deleteSettled`), past the window its moderators delete
    /// in alone (`moderatorAbilities.deleteWithin`). Only a member of the contract's seated
    /// team proposes, and the proposal is its approval. It is kept under the contract as a
    /// team action, by an id the proposer's client computes
    /// ([`ContractTeamAction::settled_deletion_action_id`](crate::data_contract::config::moderation::ContractTeamAction::settled_deletion_action_id)),
    /// which the other members approve with [`Self::ApproveTeamAction`]. The approval that
    /// meets the rule, the leader among the approvals when the rule says so, deletes the
    /// document as a `DeleteDocument` would; a rule the proposer meets alone deletes it at once.
    DeleteSettledDocument {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "documentTypeName"))]
        document_type_name: String,
        #[cfg_attr(feature = "serde-conversion", serde(rename = "documentId"))]
        document_id: Identifier,
        /// Why: stored with the proposal and with the removal record the deletion leaves.
        reason: ContractModerationReason,
    },
    /// Approves a team action another member of the contract's seated team proposed, by its
    /// id: what it does and why are the proposal's.
    ApproveTeamAction {
        #[cfg_attr(feature = "serde-conversion", serde(rename = "actionId"))]
        action_id: Identifier,
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

/// The fields of a change, each value in its canonical form: an object or JSON form does not
/// say which integer width or which binary variant the transition was built with.
#[cfg(feature = "serde-conversion")]
fn deserialize_canonical_fields<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let fields = BTreeMap::<String, Value>::deserialize(deserializer)?;
    Ok(fields
        .into_iter()
        .map(|(name, value)| {
            (
                name,
                ContractUserModerationAction::canonical_field_value(value),
            )
        })
        .collect())
}

impl ContractUserModerationAction {
    /// A field value in the one form its content has, the form clients build a change's fields
    /// in: an integer as a `U64`, or an `I64` when negative (`U128` and `I128` beyond them), 32
    /// bytes as an `Identifier` and any other binary value as `Bytes`, maps and arrays
    /// recursively. The signed bytes carry the variant, which an object or JSON form loses (a
    /// JavaScript bigint reads back as the signed width, an identifier as plain bytes), so the
    /// fields are read back in this form and built in it.
    pub fn canonical_field_value(value: Value) -> Value {
        let integer = match &value {
            Value::U128(v) => Some(i128::try_from(*v).map_err(|_| *v)),
            Value::U64(v) => Some(Ok(*v as i128)),
            Value::U32(v) => Some(Ok(*v as i128)),
            Value::U16(v) => Some(Ok(*v as i128)),
            Value::U8(v) => Some(Ok(*v as i128)),
            Value::I128(v) => Some(Ok(*v)),
            Value::I64(v) => Some(Ok(*v as i128)),
            Value::I32(v) => Some(Ok(*v as i128)),
            Value::I16(v) => Some(Ok(*v as i128)),
            Value::I8(v) => Some(Ok(*v as i128)),
            _ => None,
        };
        match integer {
            // Beyond i128, only a U128 holds it
            Some(Err(beyond)) => return Value::U128(beyond),
            Some(Ok(integer)) => {
                return if let Ok(unsigned) = u64::try_from(integer) {
                    Value::U64(unsigned)
                } else if let Ok(signed) = i64::try_from(integer) {
                    Value::I64(signed)
                } else if let Ok(unsigned) = u128::try_from(integer) {
                    Value::U128(unsigned)
                } else {
                    Value::I128(integer)
                };
            }
            None => {}
        }
        match value {
            Value::Map(entries) => Value::Map(
                entries
                    .into_iter()
                    .map(|(key, value)| {
                        (
                            Self::canonical_field_value(key),
                            Self::canonical_field_value(value),
                        )
                    })
                    .collect(),
            ),
            Value::Array(items) => {
                Value::Array(items.into_iter().map(Self::canonical_field_value).collect())
            }
            Value::Identifier(_) => value,
            value => match value.as_bytes_slice() {
                Ok(bytes) => match <[u8; 32]>::try_from(bytes) {
                    Ok(identifier) => Value::Identifier(identifier),
                    Err(_) => Value::Bytes(bytes.to_vec()),
                },
                Err(_) => value,
            },
        }
    }

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
            | ContractUserModerationAction::RestoreDocument { .. }
            | ContractUserModerationAction::ChangeDocumentFields { .. }
            | ContractUserModerationAction::DeleteSettledDocument { .. }
            | ContractUserModerationAction::ApproveTeamAction { .. } => None,
        }
    }

    /// The document a deletion targets, as its document type name and its id. `None` for an
    /// action on an identity, for a field change ([`Self::changed_document`]), for the approval
    /// of a settled document's deletion ([`Self::settled_document`]), and for a restore, which
    /// carries the document itself: its id is only known once the bytes are decoded under the
    /// document type.
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

    /// The settled document whose deletion a proposal targets, as its document type name and
    /// its id, with the reason. `None` for every other action.
    pub fn settled_document(&self) -> Option<(&str, Identifier, &ContractModerationReason)> {
        match self {
            ContractUserModerationAction::DeleteSettledDocument {
                document_type_name,
                document_id,
                reason,
            } => Some((document_type_name.as_str(), *document_id, reason)),
            _ => None,
        }
    }

    /// The team action an approval approves, by its id. `None` for every other action.
    pub fn approved_team_action(&self) -> Option<Identifier> {
        match self {
            ContractUserModerationAction::ApproveTeamAction { action_id } => Some(*action_id),
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

    /// The document a field change targets, as its document type name and its id, with the
    /// fields it sets (a `null` value removes one). `None` for every other action.
    pub fn changed_document(&self) -> Option<(&str, Identifier, &BTreeMap<String, Value>)> {
        match self {
            ContractUserModerationAction::ChangeDocumentFields {
                document_type_name,
                document_id,
                fields,
                ..
            } => Some((document_type_name.as_str(), *document_id, fields)),
            _ => None,
        }
    }

    /// The document type name a deletion, a restore, a field change or the approval of a
    /// settled document's deletion names, `None` for an action on an identity.
    pub fn document_type_name(&self) -> Option<&str> {
        match self {
            ContractUserModerationAction::DeleteDocument {
                document_type_name, ..
            }
            | ContractUserModerationAction::RestoreDocument {
                document_type_name, ..
            }
            | ContractUserModerationAction::ChangeDocumentFields {
                document_type_name, ..
            }
            | ContractUserModerationAction::DeleteSettledDocument {
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

    /// The reason a ban, a suspend, a warn, a document deletion, a field change or the approval
    /// of a settled document's deletion carries, `None` for an action that takes an identity
    /// off a list, and for a restore.
    pub fn reason(&self) -> Option<&ContractModerationReason> {
        match self {
            ContractUserModerationAction::Ban { reason, .. }
            | ContractUserModerationAction::Suspend { reason, .. }
            | ContractUserModerationAction::Warn { reason, .. }
            | ContractUserModerationAction::DeleteDocument { reason, .. }
            | ContractUserModerationAction::ChangeDocumentFields { reason, .. }
            | ContractUserModerationAction::DeleteSettledDocument { reason, .. } => Some(reason),
            ContractUserModerationAction::Unban { .. }
            | ContractUserModerationAction::Unsuspend { .. }
            | ContractUserModerationAction::ClearWarnings { .. }
            | ContractUserModerationAction::RestoreDocument { .. }
            | ContractUserModerationAction::ApproveTeamAction { .. } => None,
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
            ContractUserModerationAction::ChangeDocumentFields { .. } => "changeDocumentFields",
            ContractUserModerationAction::DeleteSettledDocument { .. } => "deleteSettledDocument",
            ContractUserModerationAction::ApproveTeamAction { .. } => "approveTeamAction",
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
            ContractUserModerationAction::DeleteSettledDocument {
                document_type_name,
                document_id,
                ..
            } => {
                write!(
                    f,
                    "propose the deletion of settled {} document {}",
                    document_type_name, document_id
                )
            }
            ContractUserModerationAction::ApproveTeamAction { action_id } => {
                write!(f, "approve team action {}", action_id)
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
            ContractUserModerationAction::ChangeDocumentFields {
                document_type_name,
                document_id,
                fields,
                ..
            } => {
                write!(
                    f,
                    "change {} of {} document {}",
                    fields.keys().cloned().collect::<Vec<_>>().join(", "),
                    document_type_name,
                    document_id
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
// reason holds a u16 and a string. The fields of a field change are document values, which
// travel in JSON as a document's own do.
#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ContractUserModerationAction {}

/// Edits the banlist, the suspension list or the warning list of a moderated data contract,
/// deletes or restores a document of one of its document types that moderators may delete,
/// approves the deletion of a settled one, or changes the fields only moderators write of one
/// of its documents. Signed by the contract owner or a moderator named in the contract's
/// config, with a CRITICAL authentication key, under the signer's contract-scoped nonce.
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

    fn field_change() -> ContractUserModerationAction {
        ContractUserModerationAction::ChangeDocumentFields {
            document_type_name: "report".to_string(),
            document_id: Identifier::from([7; 32]),
            fields: BTreeMap::from([
                ("status".to_string(), Value::U8(2)),
                ("resolution".to_string(), Value::Null),
            ]),
            reason: ContractModerationReason::from_text("handled"),
        }
    }

    #[test]
    fn should_name_the_document_and_the_fields_of_a_change() {
        let action = field_change();
        // A field change names a document, not an identity, and is not a deletion: the proof
        // of a deletion is a removal record, which a change does not leave.
        assert_eq!(action.identity_id(), None);
        assert_eq!(action.document(), None);
        assert_eq!(action.restored_document(), None);
        let (document_type_name, document_id, fields) = action
            .changed_document()
            .expect("a change names its document");
        assert_eq!(document_type_name, "report");
        assert_eq!(document_id, Identifier::from([7; 32]));
        assert_eq!(fields.len(), 2);
        assert_eq!(action.document_type_name(), Some("report"));
        assert_eq!(
            action.reason(),
            Some(&ContractModerationReason::from_text("handled"))
        );
        assert_eq!(action.name(), "changeDocumentFields");
        assert_eq!(
            action.to_string(),
            format!(
                "change resolution, status of report document {}",
                Identifier::from([7; 32])
            )
        );
    }

    #[cfg(feature = "json-conversion")]
    #[test]
    fn should_tag_a_field_change_on_the_wire() {
        let action = field_change();
        let json = serde_json::to_value(&action).expect("to json");
        assert_eq!(json["$type"], "changeDocumentFields");
        assert_eq!(json["documentTypeName"], "report");
        assert!(json.get("documentId").is_some());
        assert_eq!(json["fields"]["status"], 2);
        assert!(json["fields"]["resolution"].is_null());
        let back: ContractUserModerationAction = serde_json::from_value(json).expect("from json");
        assert_eq!(
            back.changed_document().map(|(_, id, _)| id),
            Some(Identifier::from([7; 32]))
        );
    }

    #[test]
    fn should_round_trip_a_field_change_through_bincode() {
        let transition = ContractUserModerationTransitionV0 {
            action: field_change(),
            ..make_v0()
        };
        let bytes =
            bincode::encode_to_vec(&transition, bincode::config::standard()).expect("encode");
        let (back, _): (ContractUserModerationTransitionV0, usize) =
            bincode::decode_from_slice(&bytes, bincode::config::standard()).expect("decode");
        assert_eq!(back, transition);
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
