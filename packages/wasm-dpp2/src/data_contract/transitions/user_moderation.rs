use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::serialization::{js_value_to_platform_value, platform_value_to_object};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{
    define_own_property, try_from_options, try_from_options_optional,
    try_from_options_optional_with, try_to_bytes, try_to_u16, try_to_u32, try_to_u64,
};
use dpp::data_contract::config::moderation::{
    ContractModerationDocument, ContractModerationReason, ContractWarning,
};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::platform_value::{BinaryData, Value};
use dpp::prelude::{Identifier, UserFeeIncrease};
use dpp::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationTransition;
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::v0::{
    ContractUserModerationAction, ContractUserModerationTransitionV0,
};
use dpp::state_transition::{
    StateTransition, StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned,
    StateTransitionSingleSigned,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const CONTRACT_USER_MODERATION_TS: &str = r#"
/** A document a moderation reason is about. */
export interface ContractModerationDocument {
    /** The document type of the document, on the moderated contract. */
    documentTypeName: string;
    /** The document's id, as a base58 string. */
    documentId: string;
}

/**
 * Why a moderator banned, suspended or warned an identity, or deleted a document. Every ban,
 * every suspension, every warning, every document deletion and every proposal of a settled
 * document's deletion carries one, and it is stored with the entry, the removal record or the
 * team action. Nothing checks what a moderator writes, and the documents a reason cites are not
 * looked up, except the reason document a seated elected team names, which its proposal must
 * list.
 */
export interface ContractModerationReason {
    /**
     * Reserved for the ban codes a contract may declare in a later protocol version: a u16,
     * expected to be left out today. A value is stored as written and never checked. A reason
     * read back always carries it, `null` when there is none.
     */
    code?: number | null;
    /** Free text, at most 1024 bytes of UTF-8. May be empty. */
    text: string;
    /**
     * The documents the reason is about: the posts a warning or a ban is for. At most 16,
     * none twice. A reason read back carries it only when there are any.
     */
    documents?: ContractModerationDocument[];
    /**
     * The `reason` document of the moderation charters contract the action is taken on, as a
     * base58 string. A seated elected team's ban, suspension, warning, document deletion or
     * proposal of a settled document's deletion must name one its proposal lists; for every
     * other moderator it is stored as written. A reason read back carries it only when there
     * is one.
     */
    reasonDocumentId?: string;
}

/** One warning an identity carries on a contract's warning list. */
export interface ContractWarning {
    /** The time of the block that issued it, in milliseconds. */
    warnedAt: bigint;
    /** Why, as the moderator wrote it. */
    reason: ContractModerationReason;
}

/**
 * What a moderation transition does on a contract, to one identity or to one document. The
 * wire shape of the action, as the transition's `action` field carries it.
 */
export type ContractUserModerationActionJSON =
  | { $type: "ban"; identityId: string; reason: ContractModerationReason }
  | { $type: "unban"; identityId: string }
  | { $type: "suspend"; identityId: string; until: number | string; reason: ContractModerationReason }
  | { $type: "unsuspend"; identityId: string }
  | { $type: "warn"; identityId: string; reason: ContractModerationReason }
  | { $type: "clearWarnings"; identityId: string }
  | { $type: "deleteDocument"; documentTypeName: string; documentId: string; reason: ContractModerationReason }
  | { $type: "restoreDocument"; documentTypeName: string; document: string }
  | { $type: "changeDocumentFields"; documentTypeName: string; documentId: string; fields: Record<string, unknown>; reason: ContractModerationReason }
  | { $type: "deleteSettledDocument"; documentTypeName: string; documentId: string; reason: ContractModerationReason }
  | { $type: "approveTeamAction"; actionId: string };

/**
 * Bans, unbans, suspends, unsuspends, warns or clears the warnings of one identity on a
 * moderated data contract, or deletes one document of a document type that sets
 * `moderatorAbilities.delete`, or restores one such document a moderator deleted, or sets the
 * fields a document type keeps for its moderators (`moderatorAbilities.changeFields`) on one of
 * its documents, or proposes, as a member of the contract's seated team, the deletion of a
 * settled document of a type that sets `moderatorAbilities.deleteSettled`, or approves a team
 * action another member proposed (protocol version 14).
 * Signed by the contract owner or a moderator its config names, with a CRITICAL
 * authentication key, under the signer's contract-scoped nonce.
 */
export interface ContractUserModerationTransitionOptions {
    /** The moderator that signs */
    ownerId: IdentifierLike;
    /** The moderated contract */
    dataContractId: IdentifierLike;
    /** The signer's nonce for the contract */
    identityContractNonce: bigint;
    /** What is done */
    action: "ban" | "unban" | "suspend" | "unsuspend" | "warn" | "clearWarnings" | "deleteDocument" | "restoreDocument" | "changeDocumentFields" | "deleteSettledDocument" | "approveTeamAction";
    /**
     * The identity the action targets. Needed by every action but a deleteDocument, a
     * restoreDocument, a changeDocumentFields and a deleteSettledDocument, which name a
     * document instead and refuse it, and an approveTeamAction, which names a team action.
     */
    identityId?: IdentifierLike;
    /**
     * For a deleteDocument, a restoreDocument, a changeDocumentFields or a
     * deleteSettledDocument, which need it: the document type of the document. Refused beside
     * another action.
     */
    documentTypeName?: string;
    /**
     * For a deleteDocument, a changeDocumentFields or a deleteSettledDocument, which need it:
     * the document. Refused beside another action.
     */
    documentId?: IdentifierLike;
    /**
     * For a changeDocumentFields, which needs it: each field the change sets, by its top-level
     * property name, `null` removing it. Every one must be a field the document type keeps for
     * its moderators (`moderatorAbilities.changeFields`). Refused beside another action.
     */
    fields?: Record<string, unknown>;
    /**
     * For a restoreDocument, which needs it: the document as it was serialized under its
     * document type when it was deleted (`Document.toBytes()` of the document as fetched
     * before the deletion), which must hash to what its removal record holds. Refused beside
     * another action.
     */
    document?: Uint8Array;
    /**
     * For an approveTeamAction, which needs it: the team action approved, by the id its
     * proposal returned (a deleteSettledDocument's `actionId`). Refused beside another action.
     */
    actionId?: IdentifierLike;
    /** For a suspend: the block time, in milliseconds, at which the suspension lapses */
    until?: bigint;
    /**
     * Why. A ban, a suspend, a warn and a deleteSettledDocument each need one: the proposal of
     * a settled document's deletion names a reason document the seated team's proposal lists,
     * and its approvals approve it for that reason. A deleteDocument and a changeDocumentFields
     * may leave it out, which stores no code and an empty text. Refused beside an unban, an
     * unsuspend, a clearWarnings, a restoreDocument or an approveTeamAction.
     */
    reason?: ContractModerationReason;
    userFeeIncrease?: number;
}

/**
 * ContractUserModeration serialized as a plain object.
 */
export interface ContractUserModerationObject {
    ownerId: Uint8Array;
    dataContractId: Uint8Array;
    identityContractNonce: bigint;
    action: {
        $type: string;
        identityId?: Uint8Array;
        documentTypeName?: string;
        documentId?: Uint8Array;
        document?: Uint8Array;
        fields?: Record<string, unknown>;
        actionId?: Uint8Array;
        until?: bigint;
        reason?: ContractModerationReason;
    };
    userFeeIncrease: number;
    signature?: Uint8Array;
    signaturePublicKeyId?: number;
}

/**
 * ContractUserModeration serialized as JSON.
 */
export interface ContractUserModerationJSON {
    ownerId: string;
    dataContractId: string;
    identityContractNonce: number | string;
    action: ContractUserModerationActionJSON;
    userFeeIncrease: number;
    signature?: string;
    signaturePublicKeyId?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ContractUserModerationTransitionOptions")]
    pub type ContractUserModerationTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "ContractUserModerationObject")]
    pub type ContractUserModerationObjectJs;

    #[wasm_bindgen(typescript_type = "ContractUserModerationJSON")]
    pub type ContractUserModerationJSONJs;

    #[wasm_bindgen(typescript_type = "ContractModerationReason")]
    pub type ContractModerationReasonJs;

    #[wasm_bindgen(typescript_type = "ContractWarning[]")]
    pub type ContractWarningsJs;
}

/// A reason as the plain object JavaScript reads: `code`, `null` when there is none, `text`,
/// and `documents` when it cites any, each with its type name and its id as a base58 string.
/// The shape `toJSON()` and `toObject()` give the reason inside a transition, so a reason
/// compares equal whichever call produced it.
pub fn moderation_reason_to_js(reason: &ContractModerationReason) -> JsValue {
    let object = js_sys::Object::new();
    // Setting a property on a fresh plain object cannot fail.
    let code = reason.code.map_or(JsValue::NULL, JsValue::from);
    let _ = js_sys::Reflect::set(&object, &"code".into(), &code);
    let _ = js_sys::Reflect::set(&object, &"text".into(), &JsValue::from_str(&reason.text));
    if !reason.documents.is_empty() {
        let documents = js_sys::Array::new();
        for document in &reason.documents {
            let entry = js_sys::Object::new();
            let _ = js_sys::Reflect::set(
                &entry,
                &"documentTypeName".into(),
                &JsValue::from_str(&document.document_type_name),
            );
            let _ = js_sys::Reflect::set(
                &entry,
                &"documentId".into(),
                &JsValue::from_str(&IdentifierWasm::from(document.document_id).to_base58()),
            );
            documents.push(&entry);
        }
        let _ = js_sys::Reflect::set(&object, &"documents".into(), &documents);
    }
    if let Some(reason_document_id) = reason.reason_document_id {
        let _ = js_sys::Reflect::set(
            &object,
            &"reasonDocumentId".into(),
            &JsValue::from_str(&IdentifierWasm::from(reason_document_id).to_base58()),
        );
    }
    object.into()
}

/// Warnings as the plain objects JavaScript reads, oldest first: `warnedAt` as a BigInt in
/// `toObject()` and as a number in `toJSON()`, and the reason as [`moderation_reason_to_js`]
/// gives it.
pub fn moderation_warnings_to_js(warnings: &[ContractWarning], as_json: bool) -> JsValue {
    let array = js_sys::Array::new();
    for warning in warnings {
        let object = js_sys::Object::new();
        let warned_at = if as_json {
            JsValue::from_f64(warning.warned_at as f64)
        } else {
            JsValue::from(js_sys::BigInt::from(warning.warned_at))
        };
        // Setting a property on a fresh plain object cannot fail.
        let _ = js_sys::Reflect::set(&object, &"warnedAt".into(), &warned_at);
        let _ = js_sys::Reflect::set(
            &object,
            &"reason".into(),
            &moderation_reason_to_js(&warning.reason),
        );
        array.push(&object);
    }
    array.into()
}

/// A document a reason cites, as the options of the JavaScript surfaces carry it: the id is
/// an `IdentifierLike`, which serde alone would not read from a base58 string.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractModerationDocumentInput {
    document_type_name: String,
    document_id: IdentifierWasm,
}

/// A reason as the options of the JavaScript surfaces carry it. See
/// [`ContractModerationDocumentInput`] for why it is not the reason itself.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractModerationReasonInput {
    #[serde(default)]
    code: Option<u16>,
    text: String,
    #[serde(default)]
    documents: Vec<ContractModerationDocumentInput>,
    #[serde(default)]
    reason_document_id: Option<IdentifierWasm>,
}

impl From<ContractModerationReasonInput> for ContractModerationReason {
    fn from(input: ContractModerationReasonInput) -> Self {
        ContractModerationReason {
            code: input.code,
            text: input.text,
            documents: input
                .documents
                .into_iter()
                .map(|document| ContractModerationDocument {
                    document_type_name: document.document_type_name,
                    document_id: document.document_id.into(),
                })
                .collect(),
            reason_document_id: input.reason_document_id.map(Into::into),
        }
    }
}

/// Serde struct for the primitive fields of the options
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractUserModerationOptionsInput {
    identity_contract_nonce: u64,
    action: String,
    #[serde(default)]
    document_type_name: Option<String>,
    #[serde(default)]
    until: Option<u64>,
    #[serde(default)]
    reason: Option<ContractModerationReasonInput>,
    /// `undefined` reaches serde as a unit value, so the fee is read as an option and defaulted
    #[serde(default)]
    user_fee_increase: Option<UserFeeIncrease>,
}

#[wasm_bindgen(js_name = "ContractUserModeration")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ContractUserModerationWasm(ContractUserModerationTransition);

impl From<ContractUserModerationTransition> for ContractUserModerationWasm {
    fn from(val: ContractUserModerationTransition) -> Self {
        ContractUserModerationWasm(val)
    }
}

impl From<ContractUserModerationWasm> for ContractUserModerationTransition {
    fn from(val: ContractUserModerationWasm) -> Self {
        val.0
    }
}

/// What a moderation action is made of, as the options of the JavaScript surfaces carry it.
/// Which parts an action needs and which it refuses is decided by
/// [`moderation_action_from_parts`].
#[derive(Default)]
pub struct ContractUserModerationActionParts {
    /// The identity a ban, an unban, a suspend, an unsuspend, a warn or a clearWarnings targets
    pub identity_id: Option<Identifier>,
    /// The document type of the document a deleteDocument, a restoreDocument, a
    /// changeDocumentFields or a deleteSettledDocument targets
    pub document_type_name: Option<String>,
    /// The document a deleteDocument, a changeDocumentFields or a deleteSettledDocument targets
    pub document_id: Option<Identifier>,
    /// The document a restoreDocument brings back, as it was serialized when it was deleted
    pub document: Option<Vec<u8>>,
    /// The fields a changeDocumentFields sets, a `null` removing one
    pub fields: Option<BTreeMap<String, Value>>,
    /// The team action an approveTeamAction approves
    pub action_id: Option<Identifier>,
    /// The end of a suspend
    pub until: Option<u64>,
    /// Why: needed by a ban, a suspend, a warn and a deleteSettledDocument, optional for a
    /// deleteDocument and a changeDocumentFields
    pub reason: Option<ContractModerationReason>,
}

/// A change's `fields` from JavaScript: each own property of the object a field name, its value
/// converted as a document property is (a whole number or bigint an integer, 32 bytes an
/// identifier) and put in the canonical form the transition signs
/// ([`ContractUserModerationAction::canonical_field_value`]). An `undefined` value is left out,
/// as JSON leaves it out: only an explicit `null` removes a field.
pub fn fields_from_js(value: &JsValue) -> WasmDppResult<BTreeMap<String, Value>> {
    if !value.is_object() || js_sys::Array::is_array(value) {
        return Err(WasmDppError::invalid_argument(
            "`fields` must be an object of field names to their new values",
        ));
    }
    let mut fields = BTreeMap::new();
    for entry in js_sys::Object::entries(&js_sys::Object::from(value.clone())).iter() {
        let entry = js_sys::Array::from(&entry);
        let Some(name) = entry.get(0).as_string() else {
            continue;
        };
        let field_value = entry.get(1);
        if field_value.is_undefined() {
            continue;
        }
        let field_value = js_value_to_platform_value(&field_value)
            .map_err(|error| WasmDppError::invalid_argument(format!("`fields.{name}`: {error}")))?;
        fields.insert(
            name,
            ContractUserModerationAction::canonical_field_value(field_value),
        );
    }
    Ok(fields)
}

/// The action for a name and its parts: an identity for the actions on an identity, with
/// `until` for a suspend and `reason` for a ban, a suspend and a warn, a document type
/// name and a document id for a deleteDocument, with or without a `reason`, a document
/// type name and the document's bytes for a restoreDocument, a document type name, a
/// document id and the fields for a changeDocumentFields, with or without a `reason`, a
/// document type name, a document id and a `reason` for a deleteSettledDocument, and an action
/// id for an approveTeamAction.
pub fn moderation_action_from_parts(
    action: &str,
    parts: ContractUserModerationActionParts,
) -> WasmDppResult<ContractUserModerationAction> {
    let ContractUserModerationActionParts {
        identity_id,
        document_type_name,
        document_id,
        document,
        fields,
        action_id,
        until,
        reason,
    } = parts;
    // Only a suspension ends: an `until` beside another action is refused rather than dropped,
    // or a caller asking for a timed ban would sign a permanent one.
    if until.is_some() && action != "suspend" {
        return Err(WasmDppError::invalid_argument(format!(
            "`until` is only valid for a suspend action, not for `{action}`"
        )));
    }
    // The same goes for a reason: only an action that adds an entry or a removal record stores
    // one.
    let stores_a_reason = matches!(
        action,
        "ban"
            | "suspend"
            | "warn"
            | "deleteDocument"
            | "changeDocumentFields"
            | "deleteSettledDocument"
    );
    if reason.is_some() && !stores_a_reason {
        return Err(WasmDppError::invalid_argument(format!(
            "`reason` is only valid for a ban, a suspend, a warn, a deleteDocument, a changeDocumentFields or a deleteSettledDocument action, not for `{action}`"
        )));
    }
    // Only a field change sets fields.
    if fields.is_some() && action != "changeDocumentFields" {
        return Err(WasmDppError::invalid_argument(format!(
            "`fields` is only valid for a changeDocumentFields action, not for `{action}`"
        )));
    }
    // Only an approval names a team action.
    if action_id.is_some() && action != "approveTeamAction" {
        return Err(WasmDppError::invalid_argument(format!(
            "`actionId` is only valid for an approveTeamAction action, not for `{action}`"
        )));
    }
    // And for the target: a deletion and a restore name a document, an approval a team action
    // and every other action an identity. A target of another kind is refused rather than
    // dropped, or a caller that mixed them up would sign something it did not mean. A deletion
    // names its document by id, a restore carries it.
    match action {
        "deleteDocument" | "restoreDocument" | "changeDocumentFields" | "deleteSettledDocument" => {
            if identity_id.is_some() {
                return Err(WasmDppError::invalid_argument(format!(
                    "`identityId` is not valid for a {action} action, which names a document"
                )));
            }
        }
        "approveTeamAction" => {
            if identity_id.is_some()
                || document_type_name.is_some()
                || document_id.is_some()
                || document.is_some()
            {
                return Err(WasmDppError::invalid_argument(
                    "`identityId`, `documentTypeName`, `documentId` and `document` are not valid for an approveTeamAction action, which names a team action by `actionId`: what it does is its proposal's",
                ));
            }
        }
        _ => {
            if document_type_name.is_some() || document_id.is_some() || document.is_some() {
                return Err(WasmDppError::invalid_argument(format!(
                    "`documentTypeName`, `documentId` and `document` are only valid for a deleteDocument, a restoreDocument, a changeDocumentFields or a deleteSettledDocument action, not for `{action}`"
                )));
            }
        }
    }
    if matches!(
        action,
        "deleteDocument" | "changeDocumentFields" | "deleteSettledDocument"
    ) && document.is_some()
    {
        return Err(WasmDppError::invalid_argument(format!(
            "`document` is only valid for a restoreDocument action: a {action} names its document by `documentId`"
        )));
    }
    if action == "restoreDocument" && document_id.is_some() {
        return Err(WasmDppError::invalid_argument(
            "`documentId` is only valid for a deleteDocument, a changeDocumentFields or a deleteSettledDocument action: a restoreDocument carries its document in `document`",
        ));
    }
    // What an action that lacks one of its parts is refused with.
    let needs =
        |part: &str| WasmDppError::invalid_argument(format!("a {action} action needs {part}"));
    match action {
        "ban" => Ok(ContractUserModerationAction::Ban {
            identity_id: identity_id.ok_or_else(|| needs("an `identityId`"))?,
            reason: reason.ok_or_else(|| needs("a `reason`"))?,
        }),
        "unban" => Ok(ContractUserModerationAction::Unban {
            identity_id: identity_id.ok_or_else(|| needs("an `identityId`"))?,
        }),
        "suspend" => Ok(ContractUserModerationAction::Suspend {
            identity_id: identity_id.ok_or_else(|| needs("an `identityId`"))?,
            until: until.ok_or_else(|| needs("`until`"))?,
            reason: reason.ok_or_else(|| needs("a `reason`"))?,
        }),
        "unsuspend" => Ok(ContractUserModerationAction::Unsuspend {
            identity_id: identity_id.ok_or_else(|| needs("an `identityId`"))?,
        }),
        "warn" => Ok(ContractUserModerationAction::Warn {
            identity_id: identity_id.ok_or_else(|| needs("an `identityId`"))?,
            reason: reason.ok_or_else(|| needs("a `reason`"))?,
        }),
        "clearWarnings" => Ok(ContractUserModerationAction::ClearWarnings {
            identity_id: identity_id.ok_or_else(|| needs("an `identityId`"))?,
        }),
        "deleteDocument" => Ok(ContractUserModerationAction::DeleteDocument {
            document_type_name: document_type_name.ok_or_else(|| needs("a `documentTypeName`"))?,
            document_id: document_id.ok_or_else(|| needs("a `documentId`"))?,
            // Both parts of a deletion's reason are optional, and so is the reason itself:
            // left out, the removal record stores no code and an empty text.
            reason: reason.unwrap_or_default(),
        }),
        "restoreDocument" => Ok(ContractUserModerationAction::RestoreDocument {
            document_type_name: document_type_name.ok_or_else(|| needs("a `documentTypeName`"))?,
            document: BinaryData::new(document.ok_or_else(|| needs("a `document`"))?),
        }),
        "changeDocumentFields" => Ok(ContractUserModerationAction::ChangeDocumentFields {
            document_type_name: document_type_name.ok_or_else(|| needs("a `documentTypeName`"))?,
            document_id: document_id.ok_or_else(|| needs("a `documentId`"))?,
            fields: fields.ok_or_else(|| needs("`fields`"))?,
            // As for a deletion, the reason is optional
            reason: reason.unwrap_or_default(),
        }),
        "deleteSettledDocument" => Ok(ContractUserModerationAction::DeleteSettledDocument {
            document_type_name: document_type_name.ok_or_else(|| needs("a `documentTypeName`"))?,
            document_id: document_id.ok_or_else(|| needs("a `documentId`"))?,
            // Unlike a deletion's, the reason is needed: only a seated team proposes, whose
            // reason names a reason document its proposal lists, and the approvals approve the
            // deletion for it.
            reason: reason.ok_or_else(|| needs("a `reason`"))?,
        }),
        "approveTeamAction" => Ok(ContractUserModerationAction::ApproveTeamAction {
            action_id: action_id.ok_or_else(|| needs("an `actionId`"))?,
        }),
        other => Err(WasmDppError::invalid_argument(format!(
            "unknown moderation action `{other}`: expected ban, unban, suspend, unsuspend, warn, clearWarnings, deleteDocument, restoreDocument, changeDocumentFields, deleteSettledDocument or approveTeamAction"
        ))),
    }
}

#[wasm_bindgen(js_class = ContractUserModeration)]
impl ContractUserModerationWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: ContractUserModerationTransitionOptionsJs,
    ) -> WasmDppResult<ContractUserModerationWasm> {
        // Extract complex types first (borrows &options)
        let owner_id: IdentifierWasm = try_from_options(&options, "ownerId")?;
        let data_contract_id: IdentifierWasm = try_from_options(&options, "dataContractId")?;
        let identity_id: Option<IdentifierWasm> =
            try_from_options_optional(&options, "identityId")?;
        let document_id: Option<IdentifierWasm> =
            try_from_options_optional(&options, "documentId")?;
        let action_id: Option<IdentifierWasm> = try_from_options_optional(&options, "actionId")?;
        let document = {
            let value = js_sys::Reflect::get(&options, &"document".into())
                .map_err(|_| WasmDppError::invalid_argument("failed to read `document`"))?;
            if value.is_undefined() || value.is_null() {
                None
            } else {
                Some(try_to_bytes(value, "document")?)
            }
        };
        // Read value by value, not through JSON, so a bigint stays an integer and a Uint8Array
        // bytes: the forms the `fields` getter gives back.
        let fields = try_from_options_optional_with(&options, "fields", fields_from_js)?;

        // Deserialize primitive fields via serde last (consumes options)
        let input: ContractUserModerationOptionsInput =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let action = moderation_action_from_parts(
            &input.action,
            ContractUserModerationActionParts {
                identity_id: identity_id.map(Into::into),
                document_type_name: input.document_type_name,
                document_id: document_id.map(Into::into),
                document,
                fields,
                action_id: action_id.map(Into::into),
                until: input.until,
                reason: input.reason.map(Into::into),
            },
        )?;

        Ok(ContractUserModerationWasm(
            ContractUserModerationTransition::V0(ContractUserModerationTransitionV0 {
                owner_id: owner_id.into(),
                data_contract_id: data_contract_id.into(),
                identity_contract_nonce: input.identity_contract_nonce,
                action,
                user_fee_increase: input.user_fee_increase.unwrap_or_default(),
                signature_public_key_id: 0,
                signature: Default::default(),
            }),
        ))
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<ContractUserModerationWasm> {
        let rs_transition =
            ContractUserModerationTransition::deserialize_from_bytes_untrusted(bytes.as_slice())?;

        Ok(ContractUserModerationWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<ContractUserModerationWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        ContractUserModerationWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<ContractUserModerationWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        ContractUserModerationWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(setter = "ownerId")]
    pub fn set_owner_id(&mut self, owner_id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_owner_id(owner_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "dataContractId")]
    pub fn set_data_contract_id(&mut self, id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_data_contract_id(id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "identityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, nonce: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0
            .set_identity_contract_nonce(try_to_u64(nonce, "identityContractNonce")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(setter = "signaturePublicKeyId")]
    pub fn set_signature_public_key_id(
        &mut self,
        #[wasm_bindgen(js_name = "publicKeyId")] public_key_id: &js_sys::Number,
    ) -> WasmDppResult<()> {
        self.0
            .set_signature_public_key_id(try_to_u32(public_key_id, "signaturePublicKeyId")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, amount: &js_sys::Number) -> WasmDppResult<()> {
        self.0
            .set_user_fee_increase(try_to_u16(amount, "userFeeIncrease")?);
        Ok(())
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn signature_public_key_id(&self) -> u32 {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "ownerId")]
    pub fn owner_id(&self) -> IdentifierWasm {
        use dpp::state_transition::StateTransitionOwned;
        self.0.owner_id().into()
    }

    #[wasm_bindgen(getter = "dataContractId")]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn identity_contract_nonce(&self) -> u64 {
        self.0.identity_contract_nonce()
    }

    /// The action's name: ban, unban, suspend, unsuspend, warn, clearWarnings, deleteDocument,
    /// restoreDocument, changeDocumentFields, deleteSettledDocument or approveTeamAction
    #[wasm_bindgen(getter = "action")]
    pub fn action(&self) -> String {
        self.0.action().name().to_string()
    }

    /// The identity the action targets, undefined for a deleteDocument, a restoreDocument, a
    /// changeDocumentFields and a deleteSettledDocument, which name a document whose owner is
    /// only known once the document is read, and for an approveTeamAction, which names a team
    /// action
    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> Option<IdentifierWasm> {
        self.0.target_identity_id().map(Into::into)
    }

    /// For a deleteDocument, a restoreDocument, a changeDocumentFields or a
    /// deleteSettledDocument, the document type of its document
    #[wasm_bindgen(getter = "documentTypeName")]
    pub fn document_type_name(&self) -> Option<String> {
        self.0
            .action()
            .document_type_name()
            .map(ToString::to_string)
    }

    /// For a restoreDocument, the document it brings back, as it was serialized when it was
    /// deleted
    #[wasm_bindgen(getter = "document")]
    pub fn document(&self) -> Option<Vec<u8>> {
        self.0
            .action()
            .restored_document()
            .map(|(_, document)| document.to_vec())
    }

    /// For a deleteDocument, the document it deletes, for a changeDocumentFields, the
    /// document it changes, and for a deleteSettledDocument, the document whose deletion it
    /// proposes
    #[wasm_bindgen(getter = "documentId")]
    pub fn document_id(&self) -> Option<IdentifierWasm> {
        let action = self.0.action();
        action
            .document()
            .map(|(_, document_id)| document_id)
            .or_else(|| {
                action
                    .changed_document()
                    .map(|(_, document_id, _)| document_id)
            })
            .or_else(|| {
                action
                    .settled_document()
                    .map(|(_, document_id, _)| document_id)
            })
            .map(Into::into)
    }

    /// For a changeDocumentFields, the fields it sets, in the forms the constructor takes back:
    /// integers as bigints, identifiers and bytes as Uint8Arrays, and `null` for a field it
    /// removes
    #[wasm_bindgen(
        getter = "fields",
        unchecked_return_type = "Record<string, unknown> | undefined"
    )]
    pub fn fields(&self) -> WasmDppResult<JsValue> {
        let Some((_, _, fields)) = self.0.action().changed_document() else {
            return Ok(JsValue::UNDEFINED);
        };
        let object = js_sys::Object::new();
        for (name, value) in fields {
            // A removal stays `null`: the conversion of a document's properties would read it as
            // `undefined`, which says nothing about the field.
            let value = match value {
                Value::Null => JsValue::NULL,
                value => platform_value_to_object(value)?,
            };
            // Defined as an own data property, so a field named like an inherited accessor
            // (`__proto__`) is a field, not a call to the accessor
            define_own_property(&object, name, value)?;
        }
        Ok(object.into())
    }

    /// The team action the transition proposes or approves: for a deleteSettledDocument, the
    /// id of the action it opens, computed from its contract, its owner, its nonce and the
    /// document, which the other members of the seated team approve; for an approveTeamAction,
    /// the action it approves. Undefined for every other action.
    #[wasm_bindgen(getter = "actionId")]
    pub fn action_id(&self) -> Option<IdentifierWasm> {
        self.0.team_action_id().map(Into::into)
    }

    /// For a suspend, the block time in milliseconds at which the suspension lapses
    #[wasm_bindgen(getter = "until")]
    pub fn until(&self) -> Option<u64> {
        self.0.action().until()
    }

    /// For a ban, a suspend, a warn, a deleteDocument, a changeDocumentFields and a
    /// deleteSettledDocument, why
    #[wasm_bindgen(getter = "reason")]
    pub fn reason(&self) -> Option<ContractModerationReasonJs> {
        self.0
            .action()
            .reason()
            .map(|reason| moderation_reason_to_js(reason).into())
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::from(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<ContractUserModerationWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::ContractUserModeration(st) => Ok(ContractUserModerationWasm(st)),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl ContractUserModerationWasm {
    pub fn set_signature_binary_data(&mut self, data: BinaryData) {
        self.0.set_signature(data)
    }
}

impl_wasm_conversions_inner!(
    ContractUserModerationWasm,
    ContractUserModerationTransition,
    ContractUserModeration,
    ContractUserModerationObjectJs,
    ContractUserModerationJSONJs
);

impl_wasm_type_info!(ContractUserModerationWasm, ContractUserModeration);
