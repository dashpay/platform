//! The JSON the contract moderation and fee pot functions return, one shape per read, written
//! from the rs-sdk types. Field names and meanings follow the wasm bindings
//! (`packages/wasm-sdk/src/queries/contract_moderation.rs` and
//! `packages/wasm-sdk/src/queries/contract_fee_pots.rs`): identifiers as base58 strings, times
//! in milliseconds and credits as JSON integers, the document hash as 64 hex characters. Every
//! cursor is the one rs-sdk's `after` gives; none is worked out here.

use dash_sdk::dpp::data_contract::config::moderation::ContractModerationReason;
use dash_sdk::dpp::platform_value::string_encoding::{encode, Encoding};
use dash_sdk::dpp::platform_value::{Identifier, Value};
use dash_sdk::platform::contract_fee_pots::{ContractFeePotState, ContractFeePots};
use dash_sdk::platform::contract_moderation::{
    ContractDocumentRemovalEntry, ContractDocumentRemovalsSelection,
    ContractDocumentRemovalsWithKeptValues, ContractModerationActionCounts,
    ContractModerationEntries, ContractModerationEntriesPageQuery, ContractModerationList,
    ContractModerationListStatus, ContractModerationListStatuses, ContractTeamActionEntry,
    ContractTeamActionEvent, ContractTeamActionSigners, ContractTeamActions,
    ContractTeamActionsPageQuery, ContractWarning,
};
use serde::ser::{Error as _, SerializeMap};
use serde::{Serialize, Serializer};
use serde_json::{json, Map, Value as JsonValue};
use std::collections::BTreeMap;

fn base58(id: Identifier) -> String {
    id.to_string(Encoding::Base58)
}

/// The name a list is written with: `"banlist"`, `"suspensions"` or `"warnings"`.
pub(super) fn list_name(list: ContractModerationList) -> &'static str {
    match list {
        ContractModerationList::Banlist => "banlist",
        ContractModerationList::Suspensions => "suspensions",
        ContractModerationList::Warnings => "warnings",
    }
}

/// A reason: `code` always present (null when there is none), `documents` and
/// `reasonDocumentId` only when the reason carries them.
pub(super) fn reason_json(reason: &ContractModerationReason) -> JsonValue {
    let mut object = Map::new();
    object.insert("code".to_string(), json!(reason.code));
    object.insert("text".to_string(), json!(reason.text));
    if !reason.documents.is_empty() {
        let documents = reason
            .documents
            .iter()
            .map(|document| {
                json!({
                    "documentTypeName": document.document_type_name,
                    "documentId": base58(document.document_id),
                })
            })
            .collect();
        object.insert("documents".to_string(), JsonValue::Array(documents));
    }
    if let Some(reason_document_id) = reason.reason_document_id {
        object.insert(
            "reasonDocumentId".to_string(),
            json!(base58(reason_document_id)),
        );
    }
    JsonValue::Object(object)
}

/// Warnings, oldest first.
fn warnings_json(warnings: &[ContractWarning]) -> JsonValue {
    warnings
        .iter()
        .map(|warning| {
            json!({
                "warnedAt": warning.warned_at,
                "reason": reason_json(&warning.reason),
            })
        })
        .collect()
}

/// One identity's status on the lists read: `lists`, and the fields of those lists only, so a
/// list not read stays absent (unknown) rather than reading as "not banned".
pub(super) fn status_json(statuses: &ContractModerationListStatuses) -> JsonValue {
    let mut object = Map::new();
    let mut lists = Vec::new();
    for status in &statuses.0 {
        lists.push(json!(list_name(status.list())));
        match status {
            ContractModerationListStatus::Banlist { ban } => {
                object.insert("banned".to_string(), json!(ban.is_some()));
                if let Some(ban) = ban {
                    object.insert("banReason".to_string(), reason_json(&ban.reason));
                }
            }
            ContractModerationListStatus::Suspensions { suspension } => {
                if let Some(suspension) = suspension {
                    object.insert("suspendedUntil".to_string(), json!(suspension.until));
                    object.insert(
                        "suspensionReason".to_string(),
                        reason_json(&suspension.reason),
                    );
                }
            }
            ContractModerationListStatus::Warnings { warnings } => {
                object.insert("warnings".to_string(), warnings_json(warnings));
            }
        }
    }
    object.insert("lists".to_string(), JsonValue::Array(lists));
    JsonValue::Object(object)
}

/// One page of a moderation list, with `nextStartAfter` when `query.after(page)` gives a next
/// page.
pub(super) fn entries_json(
    page: &ContractModerationEntries,
    query: &ContractModerationEntriesPageQuery,
) -> JsonValue {
    let entries = page
        .entries()
        .iter()
        .map(|entry| {
            let mut object = Map::new();
            object.insert("identityId".to_string(), json!(base58(entry.identity_id)));
            if let Some(until) = entry.until {
                object.insert("until".to_string(), json!(until));
            }
            object.insert("reason".to_string(), reason_json(&entry.reason));
            if !entry.warnings.is_empty() {
                object.insert("warnings".to_string(), warnings_json(&entry.warnings));
            }
            JsonValue::Object(object)
        })
        .collect();
    let mut object = Map::new();
    object.insert("entries".to_string(), JsonValue::Array(entries));
    if let Some(start_after) = query.after(page).and_then(|next| next.query.start_after) {
        object.insert("nextStartAfter".to_string(), json!(base58(start_after)));
    }
    JsonValue::Object(object)
}

/// A kept platform value written in the tagged form `{"type": ..., "value": ...}`, its kind
/// carried by the tag rather than inferred from the JSON number, so a reader keeps a signed and
/// an unsigned integer apart and an integral or negative-zero float a float:
/// * `null` (no `value`), `bool`, `string`;
/// * `int` for a signed integer (`i8` to `i64`, and an `i128` within `i64`), `uint` for an
///   unsigned one (`u8` to `u64`, and a `u128` within `u64`), each a JSON integer; `int128` and
///   `uint128` for a 128-bit value outside that range, as a decimal string;
/// * `double` for a float, a JSON number written with its fraction and sign (`1.0`, `-0.0`);
/// * `identifier` (base58) and `bytes` (base64, for every byte-string variant);
/// * `array` of tagged values, and `object` of tagged values by member name.
///
/// It serializes straight into the output, borrowing the value: a page can keep about a
/// million values, and building an intermediate `serde_json::Value` tree for them first would
/// take many times the memory of the string it becomes.
pub(super) struct KeptValue<'a>(pub(super) &'a Value);

impl Serialize for KeptValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Null => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("type", "null")?;
                map.end()
            }
            Value::Bool(value) => tagged(serializer, "bool", value),
            Value::Text(value) => tagged(serializer, "string", value),
            Value::I8(value) => tagged(serializer, "int", value),
            Value::I16(value) => tagged(serializer, "int", value),
            Value::I32(value) => tagged(serializer, "int", value),
            Value::I64(value) => tagged(serializer, "int", value),
            Value::I128(value) => match i64::try_from(*value) {
                Ok(value) => tagged(serializer, "int", &value),
                Err(_) => tagged(serializer, "int128", &value.to_string()),
            },
            Value::U8(value) => tagged(serializer, "uint", value),
            Value::U16(value) => tagged(serializer, "uint", value),
            Value::U32(value) => tagged(serializer, "uint", value),
            Value::U64(value) => tagged(serializer, "uint", value),
            Value::U128(value) => match u64::try_from(*value) {
                Ok(value) => tagged(serializer, "uint", &value),
                Err(_) => tagged(serializer, "uint128", &value.to_string()),
            },
            Value::Float(value) if value.is_finite() => tagged(serializer, "double", value),
            Value::Float(value) => Err(S::Error::custom(format!("{value} is not a finite float"))),
            Value::Identifier(bytes) => {
                tagged(serializer, "identifier", &encode(bytes, Encoding::Base58))
            }
            Value::Bytes(bytes) => tagged(serializer, "bytes", &encode(bytes, Encoding::Base64)),
            Value::Bytes20(bytes) => tagged(serializer, "bytes", &encode(bytes, Encoding::Base64)),
            Value::Bytes32(bytes) => tagged(serializer, "bytes", &encode(bytes, Encoding::Base64)),
            Value::Bytes36(bytes) => tagged(serializer, "bytes", &encode(bytes, Encoding::Base64)),
            Value::Array(items) => tagged(serializer, "array", &KeptItems(items)),
            Value::Map(members) => tagged(serializer, "object", &KeptMembers(members)),
            // No document property is stored as an enum value, so no record keeps one
            Value::EnumU8(_) | Value::EnumString(_) => {
                Err(S::Error::custom("an enum value is not a kept field"))
            }
            // A variant added to platform values later is refused until it gets a tag here
            other => Err(S::Error::custom(format!("{other:?} has no kept field tag"))),
        }
    }
}

/// `{"type": tag, "value": value}`, the value serialized in place.
fn tagged<S: Serializer, T: Serialize + ?Sized>(
    serializer: S,
    tag: &str,
    value: &T,
) -> Result<S::Ok, S::Error> {
    let mut map = serializer.serialize_map(Some(2))?;
    map.serialize_entry("type", tag)?;
    map.serialize_entry("value", value)?;
    map.end()
}

/// The items of a kept array, each tagged.
struct KeptItems<'a>(&'a [Value]);

impl Serialize for KeptItems<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(KeptValue))
    }
}

/// The members of a kept object, each tagged, in their order; a member named by anything but
/// text is refused.
struct KeptMembers<'a>(&'a [(Value, Value)]);

impl Serialize for KeptMembers<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in self.0 {
            let Value::Text(key) = key else {
                return Err(S::Error::custom(format!(
                    "an object member is named by {key:?}, not by text"
                )));
            };
            map.serialize_entry(key, &KeptValue(value))?;
        }
        map.end()
    }
}

/// Removal records with their kept values, serialized straight into the output: `removals`,
/// then `nextStartAfter` when rs-sdk gives a next page (never for a read by ids).
pub(super) struct RemovalsPageJson(ContractDocumentRemovalsWithKeptValues);

/// The removals page of `removals`, ready to serialize.
pub(super) fn removals_json(removals: ContractDocumentRemovalsWithKeptValues) -> RemovalsPageJson {
    RemovalsPageJson(removals)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemovalsPageView<'a> {
    removals: Vec<RemovalEntryView<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_start_after: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemovalEntryView<'a> {
    document_id: String,
    document_owner_id: String,
    moderator_id: String,
    reason: JsonValue,
    removed_at: u64,
    document_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    restored_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    restored_at: Option<u64>,
    kept_fields: BTreeMap<&'a str, KeptValue<'a>>,
}

impl Serialize for RemovalsPageJson {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let next_start_after = self
            .0
            .next
            .as_ref()
            .and_then(|next| match &next.query.selection {
                ContractDocumentRemovalsSelection::Page { start_after, .. } => *start_after,
                ContractDocumentRemovalsSelection::DocumentIds(_) => None,
            });
        RemovalsPageView {
            removals: self
                .0
                .removals
                .iter()
                .map(|(entry, kept_values)| removal_entry_view(entry, kept_values))
                .collect(),
            next_start_after: next_start_after.map(base58),
        }
        .serialize(serializer)
    }
}

fn removal_entry_view<'a>(
    entry: &'a ContractDocumentRemovalEntry,
    kept_values: &'a BTreeMap<String, Value>,
) -> RemovalEntryView<'a> {
    let removal = &entry.removal;
    RemovalEntryView {
        document_id: base58(entry.document_id),
        document_owner_id: base58(removal.document_owner_id),
        moderator_id: base58(removal.moderator_id),
        reason: reason_json(&removal.reason),
        removed_at: removal.removed_at,
        document_hash: hex::encode(removal.document_hash),
        restored_by: removal
            .restoration
            .as_ref()
            .map(|restoration| base58(restoration.moderator_id)),
        restored_at: removal
            .restoration
            .as_ref()
            .map(|restoration| restoration.restored_at),
        kept_fields: kept_values
            .iter()
            .map(|(path, value)| (path.as_str(), KeptValue(value)))
            .collect(),
    }
}

fn team_action_json(entry: &ContractTeamActionEntry) -> JsonValue {
    let event = match &entry.action.event {
        ContractTeamActionEvent::DeleteSettledDocument {
            document_type_name,
            document_id,
            document_last_modified_at,
            document_revision,
            reason,
        } => {
            let mut event = Map::new();
            event.insert("type".to_string(), json!("deleteSettledDocument"));
            event.insert("documentTypeName".to_string(), json!(document_type_name));
            event.insert("documentId".to_string(), json!(base58(*document_id)));
            event.insert(
                "documentLastModifiedAt".to_string(),
                json!(document_last_modified_at),
            );
            if let Some(revision) = document_revision {
                event.insert("documentRevision".to_string(), json!(revision));
            }
            event.insert("reason".to_string(), reason_json(reason));
            JsonValue::Object(event)
        }
    };
    json!({
        "actionId": base58(entry.action_id),
        "proposerId": base58(entry.action.proposer_id),
        "proposedAt": entry.action.proposed_at,
        "event": event,
        "approvalCount": entry.approval_count,
    })
}

/// One page of team actions, with `nextStartAtActionId` and `nextStartAtActionIdIncluded` when
/// `query.after(page)` gives a next page.
pub(super) fn team_actions_json(
    page: &ContractTeamActions,
    query: &ContractTeamActionsPageQuery,
) -> JsonValue {
    let actions = page.actions().iter().map(team_action_json).collect();
    let mut object = Map::new();
    object.insert("actions".to_string(), JsonValue::Array(actions));
    if let Some((action_id, included)) = query.after(page).and_then(|next| next.query.start_at) {
        object.insert("nextStartAtActionId".to_string(), json!(base58(action_id)));
        object.insert("nextStartAtActionIdIncluded".to_string(), json!(included));
    }
    JsonValue::Object(object)
}

pub(super) fn team_action_signers_json(signers: &ContractTeamActionSigners) -> JsonValue {
    let signer_ids: Vec<String> = signers.signers().iter().copied().map(base58).collect();
    json!({ "signerIds": signer_ids })
}

pub(super) fn action_counts_json(counts: &ContractModerationActionCounts) -> JsonValue {
    let counts: Vec<JsonValue> = counts
        .counts()
        .iter()
        .map(|(identity_id, count)| json!({"identityId": base58(*identity_id), "count": count}))
        .collect();
    json!({ "counts": counts })
}

/// One pot; the last claim's fields only when the pot was ever claimed (epoch 0 is an epoch a
/// pot can have been claimed in, so "never" is the absent fields).
fn fee_pot_json(pot: &ContractFeePotState) -> JsonValue {
    let mut object = Map::new();
    object.insert("credits".to_string(), json!(pot.credits));
    if let Some(last_claim) = pot.last_claim {
        object.insert("lastClaimEpoch".to_string(), json!(last_claim.epoch_index));
        object.insert("lastClaimTimeMs".to_string(), json!(last_claim.time_ms));
        object.insert(
            "lastClaimantId".to_string(),
            json!(base58(last_claim.claimant_id)),
        );
    }
    JsonValue::Object(object)
}

pub(super) fn fee_pots_json(pots: &ContractFeePots) -> JsonValue {
    json!({
        "owner": fee_pot_json(&pots.owner),
        "moderators": fee_pot_json(&pots.moderators),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::data_contract::config::moderation::ContractDocumentRestoration;
    use dash_sdk::dpp::data_contract::config::moderation::{
        ContractBan, ContractModerationDocument, ContractSuspension,
    };
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::platform::contract_fee_pots::ContractFeePotLastClaim;
    use dash_sdk::platform::contract_moderation::{
        ContractDocumentRemoval, ContractDocumentRemovalsPageQuery, ContractModerationEntry,
        ContractTeamAction, GroupActionStatus,
    };

    fn id(seed: u8) -> Identifier {
        Identifier::new([seed; 32])
    }

    fn b58(seed: u8) -> String {
        base58(id(seed))
    }

    /// A reason citing a document and naming a reason document.
    fn full_reason() -> ContractModerationReason {
        ContractModerationReason {
            code: Some(7),
            text: "spam".to_string(),
            documents: vec![ContractModerationDocument {
                document_type_name: "post".to_string(),
                document_id: id(5),
            }],
            reason_document_id: Some(id(6)),
        }
    }

    fn full_reason_json() -> JsonValue {
        json!({
            "code": 7,
            "text": "spam",
            "documents": [{"documentTypeName": "post", "documentId": b58(5)}],
            "reasonDocumentId": b58(6),
        })
    }

    #[test]
    fn should_write_a_reason_with_a_null_code_and_without_absent_parts() {
        assert_eq!(
            reason_json(&ContractModerationReason::from_text("spam")),
            json!({"code": null, "text": "spam"})
        );
        assert_eq!(reason_json(&full_reason()), full_reason_json());
    }

    #[test]
    fn should_write_only_the_lists_read_in_a_status() {
        let statuses = ContractModerationListStatuses(vec![
            ContractModerationListStatus::Banlist {
                ban: Some(ContractBan {
                    reason: full_reason(),
                }),
            },
            ContractModerationListStatus::Suspensions {
                suspension: Some(ContractSuspension {
                    until: 1_700_000_000_000,
                    reason: ContractModerationReason::from_text("cool off"),
                }),
            },
            ContractModerationListStatus::Warnings {
                warnings: vec![ContractWarning {
                    warned_at: 1_600_000_000_000,
                    reason: ContractModerationReason::from_text("first"),
                }],
            },
        ]);
        assert_eq!(
            status_json(&statuses),
            json!({
                "lists": ["banlist", "suspensions", "warnings"],
                "banned": true,
                "banReason": full_reason_json(),
                "suspendedUntil": 1_700_000_000_000u64,
                "suspensionReason": {"code": null, "text": "cool off"},
                "warnings": [{"warnedAt": 1_600_000_000_000u64, "reason": {"code": null, "text": "first"}}],
            })
        );

        // Not banned, read on the banlist alone: no suspension or warning fields at all
        let banlist_only =
            ContractModerationListStatuses(vec![ContractModerationListStatus::Banlist {
                ban: None,
            }]);
        assert_eq!(
            status_json(&banlist_only),
            json!({"lists": ["banlist"], "banned": false})
        );

        // Not suspended and never warned, on the lists read
        let clear = ContractModerationListStatuses(vec![
            ContractModerationListStatus::Suspensions { suspension: None },
            ContractModerationListStatus::Warnings { warnings: vec![] },
        ]);
        assert_eq!(
            status_json(&clear),
            json!({"lists": ["suspensions", "warnings"], "warnings": []})
        );
    }

    fn entries_query(limit: u16) -> ContractModerationEntriesPageQuery {
        ContractModerationEntriesPageQuery::new(
            id(1),
            ContractModerationList::Suspensions,
            PlatformVersion::latest(),
        )
        .with_limit(limit)
    }

    fn suspension_entry(seed: u8) -> ContractModerationEntry {
        ContractModerationEntry {
            identity_id: id(seed),
            until: Some(1_700_000_000_000),
            reason: ContractModerationReason::from_text("cool off"),
            warnings: vec![],
        }
    }

    #[test]
    fn should_write_an_entries_page_with_the_cursor_rs_sdk_gives() {
        let page = ContractModerationEntries(vec![suspension_entry(3), suspension_entry(4)]);
        assert_eq!(
            entries_json(&page, &entries_query(2)),
            json!({
                "entries": [
                    {"identityId": b58(3), "until": 1_700_000_000_000u64, "reason": {"code": null, "text": "cool off"}},
                    {"identityId": b58(4), "until": 1_700_000_000_000u64, "reason": {"code": null, "text": "cool off"}},
                ],
                "nextStartAfter": b58(4),
            })
        );
        // A page shorter than the limit is the last: no cursor
        assert_eq!(
            entries_json(&page, &entries_query(3)),
            json!({
                "entries": [
                    {"identityId": b58(3), "until": 1_700_000_000_000u64, "reason": {"code": null, "text": "cool off"}},
                    {"identityId": b58(4), "until": 1_700_000_000_000u64, "reason": {"code": null, "text": "cool off"}},
                ],
            })
        );
    }

    #[test]
    fn should_write_a_warning_entry_with_its_warnings() {
        let page = ContractModerationEntries(vec![ContractModerationEntry {
            identity_id: id(3),
            until: None,
            reason: ContractModerationReason::from_text("second"),
            warnings: vec![
                ContractWarning {
                    warned_at: 1,
                    reason: ContractModerationReason::from_text("first"),
                },
                ContractWarning {
                    warned_at: 2,
                    reason: ContractModerationReason::from_text("second"),
                },
            ],
        }]);
        assert_eq!(
            entries_json(&page, &entries_query(100)),
            json!({
                "entries": [{
                    "identityId": b58(3),
                    "reason": {"code": null, "text": "second"},
                    "warnings": [
                        {"warnedAt": 1, "reason": {"code": null, "text": "first"}},
                        {"warnedAt": 2, "reason": {"code": null, "text": "second"}},
                    ],
                }],
            })
        );
    }

    fn removal_entry(seed: u8, restored: bool) -> ContractDocumentRemovalEntry {
        ContractDocumentRemovalEntry {
            document_id: id(seed),
            removal: ContractDocumentRemoval {
                document_owner_id: id(20),
                moderator_id: id(21),
                reason: ContractModerationReason::from_text("off topic"),
                removed_at: 1_700_000_000_000,
                document_hash: [0xab; 32],
                restoration: restored.then_some(ContractDocumentRestoration {
                    moderator_id: id(22),
                    restored_at: 1_700_000_100_000,
                }),
                kept_fields: vec![],
            },
        }
    }

    #[test]
    fn should_write_removals_with_their_kept_fields_and_cursor() {
        let next = ContractDocumentRemovalsPageQuery::new(
            id(1),
            "post".to_string(),
            PlatformVersion::latest(),
        );
        let mut next = next.with_limit(2);
        next.query.selection = ContractDocumentRemovalsSelection::Page {
            start_after: Some(id(4)),
            limit: 2,
        };
        let removals = ContractDocumentRemovalsWithKeptValues {
            removals: vec![
                (
                    removal_entry(3, false),
                    BTreeMap::from([
                        ("$createdAt".to_string(), Value::U64(1_600_000_000_000)),
                        ("author".to_string(), Value::Identifier([9; 32])),
                        ("hashtag".to_string(), Value::Text("dash".to_string())),
                        (
                            "meta".to_string(),
                            Value::Map(vec![(
                                Value::Text("tags".to_string()),
                                Value::Array(vec![Value::Text("privacy".to_string())]),
                            )]),
                        ),
                        ("score".to_string(), Value::I64(-3)),
                    ]),
                ),
                (removal_entry(4, true), BTreeMap::new()),
            ],
            next: Some(next),
        };
        assert_eq!(
            serde_json::to_value(removals_json(removals)).expect("writes"),
            json!({
                "removals": [
                    {
                        "documentId": b58(3),
                        "documentOwnerId": b58(20),
                        "moderatorId": b58(21),
                        "reason": {"code": null, "text": "off topic"},
                        "removedAt": 1_700_000_000_000u64,
                        "documentHash": "ab".repeat(32),
                        "keptFields": {
                            "$createdAt": {"type": "uint", "value": 1_600_000_000_000u64},
                            "author": {"type": "identifier", "value": b58(9)},
                            "hashtag": {"type": "string", "value": "dash"},
                            "meta": {"type": "object", "value": {
                                "tags": {"type": "array", "value": [
                                    {"type": "string", "value": "privacy"},
                                ]},
                            }},
                            "score": {"type": "int", "value": -3},
                        },
                    },
                    {
                        "documentId": b58(4),
                        "documentOwnerId": b58(20),
                        "moderatorId": b58(21),
                        "reason": {"code": null, "text": "off topic"},
                        "removedAt": 1_700_000_000_000u64,
                        "documentHash": "ab".repeat(32),
                        "restoredBy": b58(22),
                        "restoredAt": 1_700_000_100_000u64,
                        "keptFields": {},
                    },
                ],
                "nextStartAfter": b58(4),
            })
        );
    }

    #[test]
    fn should_write_removals_read_by_ids_without_a_cursor() {
        let removals = ContractDocumentRemovalsWithKeptValues {
            removals: vec![(removal_entry(3, false), BTreeMap::new())],
            next: None,
        };
        let json = serde_json::to_value(removals_json(removals)).expect("writes");
        assert_eq!(json["removals"].as_array().map(Vec::len), Some(1));
        assert!(json.get("nextStartAfter").is_none());
    }

    fn team_action_entry(seed: u8, revision: Option<u64>) -> ContractTeamActionEntry {
        ContractTeamActionEntry {
            action_id: id(seed),
            action: ContractTeamAction {
                proposer_id: id(30),
                proposed_at: 1_000,
                event: ContractTeamActionEvent::DeleteSettledDocument {
                    document_type_name: "post".to_string(),
                    document_id: id(31),
                    document_last_modified_at: 10,
                    document_revision: revision,
                    reason: ContractModerationReason::from_text("spam"),
                },
            },
            approval_count: 2,
        }
    }

    #[test]
    fn should_write_a_team_actions_page_with_the_cursor_rs_sdk_gives() {
        let query = ContractTeamActionsPageQuery::new(
            id(1),
            GroupActionStatus::ActionActive,
            PlatformVersion::latest(),
        )
        .with_limit(2);
        let page = ContractTeamActions(vec![
            team_action_entry(3, Some(4)),
            team_action_entry(4, None),
        ]);
        assert_eq!(
            team_actions_json(&page, &query),
            json!({
                "actions": [
                    {
                        "actionId": b58(3),
                        "proposerId": b58(30),
                        "proposedAt": 1_000,
                        "event": {
                            "type": "deleteSettledDocument",
                            "documentTypeName": "post",
                            "documentId": b58(31),
                            "documentLastModifiedAt": 10,
                            "documentRevision": 4,
                            "reason": {"code": null, "text": "spam"},
                        },
                        "approvalCount": 2,
                    },
                    {
                        "actionId": b58(4),
                        "proposerId": b58(30),
                        "proposedAt": 1_000,
                        "event": {
                            "type": "deleteSettledDocument",
                            "documentTypeName": "post",
                            "documentId": b58(31),
                            "documentLastModifiedAt": 10,
                            "reason": {"code": null, "text": "spam"},
                        },
                        "approvalCount": 2,
                    },
                ],
                "nextStartAtActionId": b58(4),
                "nextStartAtActionIdIncluded": false,
            })
        );
        let short = ContractTeamActions(vec![team_action_entry(3, None)]);
        assert!(team_actions_json(&short, &query)
            .get("nextStartAtActionId")
            .is_none());
    }

    #[test]
    fn should_write_signers_and_action_counts_in_identity_order() {
        assert_eq!(
            team_action_signers_json(&ContractTeamActionSigners(vec![id(1), id(2)])),
            json!({"signerIds": [b58(1), b58(2)]})
        );
        assert_eq!(
            team_action_signers_json(&ContractTeamActionSigners::default()),
            json!({"signerIds": []})
        );
        let counts = ContractModerationActionCounts(BTreeMap::from([(id(2), 5), (id(1), 3)]));
        assert_eq!(
            action_counts_json(&counts),
            json!({"counts": [
                {"identityId": b58(1), "count": 3},
                {"identityId": b58(2), "count": 5},
            ]})
        );
    }

    #[test]
    fn should_write_fee_pots_with_a_last_claim_only_when_paid_out() {
        let pots = ContractFeePots {
            owner: ContractFeePotState {
                credits: 10_000_000,
                last_claim: None,
            },
            moderators: ContractFeePotState {
                credits: u64::MAX,
                last_claim: Some(ContractFeePotLastClaim {
                    epoch_index: 0,
                    time_ms: 1_700_000_000_000,
                    claimant_id: id(9),
                }),
            },
        };
        assert_eq!(
            fee_pots_json(&pots),
            json!({
                "owner": {"credits": 10_000_000},
                "moderators": {
                    "credits": u64::MAX,
                    "lastClaimEpoch": 0,
                    "lastClaimTimeMs": 1_700_000_000_000u64,
                    "lastClaimantId": b58(9),
                },
            })
        );
    }

    /// The kept values the Swift decoding tests read too
    /// (`ContractModerationDecodingTests.testShouldDecodeTheSharedKeptFieldsFixtureByItsTags`):
    /// one file for both, written compact, one line.
    const KEPT_FIELDS_FIXTURE: &str = include_str!("fixtures/kept_fields.json");

    #[test]
    fn should_write_kept_values_tagged_with_their_kind() {
        let kept = BTreeMap::from([
            ("bytes".to_string(), Value::Bytes(vec![1, 2, 3])),
            ("flag".to_string(), Value::Bool(true)),
            ("i64min".to_string(), Value::I64(i64::MIN)),
            ("id".to_string(), Value::Identifier([9; 32])),
            ("int128".to_string(), Value::I128(i128::MIN)),
            ("negativeZero".to_string(), Value::Float(-0.0)),
            (
                "nested".to_string(),
                Value::Map(vec![(
                    Value::Text("list".to_string()),
                    Value::Array(vec![
                        Value::U8(7),
                        Value::Null,
                        Value::Text("a".to_string()),
                    ]),
                )]),
            ),
            ("u64max".to_string(), Value::U64(u64::MAX)),
            ("uint128".to_string(), Value::U128(u128::MAX)),
            ("whole".to_string(), Value::Float(1.0)),
        ]);
        let written: BTreeMap<&str, KeptValue> = kept
            .iter()
            .map(|(path, value)| (path.as_str(), KeptValue(value)))
            .collect();

        // The fixture byte for byte: the keys sort, and the floats keep their fraction and sign
        assert_eq!(
            serde_json::to_string(&written).expect("serializes"),
            KEPT_FIELDS_FIXTURE.trim_end()
        );
    }

    #[test]
    fn should_write_an_in_range_128_bit_value_by_its_signedness() {
        assert_eq!(
            serde_json::to_value(KeptValue(&Value::I128(-5))).expect("writes"),
            json!({"type": "int", "value": -5})
        );
        assert_eq!(
            serde_json::to_value(KeptValue(&Value::U128(5))).expect("writes"),
            json!({"type": "uint", "value": 5})
        );
        // The same magnitude, signed and unsigned, stays apart
        assert_eq!(
            serde_json::to_value(KeptValue(&Value::I64(5))).expect("writes"),
            json!({"type": "int", "value": 5})
        );
        assert_eq!(
            serde_json::to_value(KeptValue(&Value::U64(5))).expect("writes"),
            json!({"type": "uint", "value": 5})
        );
        assert!(serde_json::to_string(&KeptValue(&Value::Float(f64::NAN))).is_err());
        assert!(serde_json::to_string(&KeptValue(&Value::EnumU8(vec![1]))).is_err());
    }

    #[test]
    fn should_refuse_a_kept_object_member_not_named_by_text() {
        let value = Value::Map(vec![(Value::U8(1), Value::Null)]);
        assert!(serde_json::to_string(&KeptValue(&value)).is_err());
    }

    /// A record keeping a 1,024-element boolean array writes exactly this string.
    #[test]
    fn should_write_a_record_keeping_a_large_array_exactly() {
        let removals = ContractDocumentRemovalsWithKeptValues {
            removals: vec![(
                removal_entry(3, false),
                BTreeMap::from([(
                    "flags".to_string(),
                    Value::Array(vec![Value::Bool(true); 1024]),
                )]),
            )],
            next: None,
        };
        let flags = vec![r#"{"type":"bool","value":true}"#; 1024].join(",");
        let expected = format!(
            r#"{{"removals":[{{"documentId":"{}","documentOwnerId":"{}","moderatorId":"{}","reason":{{"code":null,"text":"off topic"}},"removedAt":1700000000000,"documentHash":"{}","keptFields":{{"flags":{{"type":"array","value":[{flags}]}}}}}}]}}"#,
            b58(3),
            b58(20),
            b58(21),
            "ab".repeat(32),
        );

        assert_eq!(
            serde_json::to_string(&removals_json(removals)).expect("writes"),
            expected
        );
    }
}
