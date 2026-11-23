//! FFI bindings for moderating a data contract's users and documents on
//! `IdentityWallet` (protocol version 14).
//!
//! Thin: each function reads its C arguments, calls one `IdentityWallet`
//! method (`contract_moderation_offer`,
//! `moderate_contract_user_with_signer`,
//! `delete_contract_document_with_signer`,
//! `restore_contract_document_with_signer`,
//! `change_contract_document_fields_with_signer`,
//! `propose_settled_contract_document_deletion_with_signer`,
//! `approve_contract_team_action_with_signer`) on the platform-wallet 8 MB
//! worker stack, and writes the result back as JSON. rs-sdk, the wallet and
//! Drive decide everything else: the signing key, who may moderate what,
//! and whether the target, document or team action admits the moderation.
//!
//! # The action argument
//!
//! The moderation of an identity crosses the boundary as a plain `uint8_t`
//! ([`CONTRACT_USER_MODERATION_BAN`] to
//! [`CONTRACT_USER_MODERATION_CLEAR_WARNINGS`]). Any other byte is refused
//! with `ErrorInvalidParameter` before the wallet is looked up, rather than
//! read as a `#[repr(u8)]` enum (an out-of-range value would be undefined
//! behavior), as the fee claim takes its pot byte.
//!
//! # Reasons
//!
//! A reason crosses as two nullable arguments: `reason_text`, a
//! NUL-terminated UTF-8 string (null for an empty text), and
//! `reason_document_id`, 32 bytes naming a `reason` document of the
//! moderation charters contract (null for none), which a seated team's
//! moderation must name from its proposal's list. The reversals (unban,
//! unsuspend, clear warnings) carry no reason and ignore both.
//!
//! # JSON
//!
//! Identifiers are base58 strings, times milliseconds, as the rs-sdk-ffi
//! moderation reads write them. A target's status is written by the status
//! read's own writer (`rs_sdk_ffi::contract_moderation_status_json`) and a
//! removal record by the removals read's
//! (`rs_sdk_ffi::contract_document_removal_json`), so a host decodes them
//! with the types it decodes the reads with. A team action's status is
//! `"active"` or `"closed"`, as the team action reads name it.
//!
//! # Panics
//!
//! Each export runs its body under `catch_panic_to_code`, so a panic inside
//! (`block_on_worker` re-raises a worker's) is a result, not an abort, where
//! panics unwind. The offer read reports `ErrorWalletOperation`; a write
//! reports `ErrorTransactionBroadcastUnconfirmed`, since it may have been
//! broadcast before the panic. Every export checks and nulls `out_json`
//! before any other check, so every error leaves it null.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dash_sdk::platform::transition::contract_user_moderation::ModeratedUserStatus;
use dpp::data_contract::config::moderation::{ContractModerationReason, SettledDeletionRule};
use dpp::document::Document;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::platform_value::string_encoding::{encode, Encoding};
use dpp::prelude::Identifier;
use platform_wallet::{
    ContractDocumentDeletion, ContractDocumentRestore, ContractModerationOffer,
    ContractModeratorRole, ContractUserModerationKind, DocumentTypeModerationOffer,
    PlatformWalletError,
};
use rs_sdk_ffi::{
    contract_document_removal_json, contract_moderation_status_json, SignerHandle, VTableSigner,
};
use serde_json::{json, Map, Value as JsonValue};

use crate::check_ptr;
use crate::contract_fee_claim::moderators_pot_team_name;
use crate::document::confirmed_document_to_json;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// The `action` byte of a ban.
pub const CONTRACT_USER_MODERATION_BAN: u8 = 0;
/// The `action` byte of an unban.
pub const CONTRACT_USER_MODERATION_UNBAN: u8 = 1;
/// The `action` byte of a suspension.
pub const CONTRACT_USER_MODERATION_SUSPEND: u8 = 2;
/// The `action` byte of an unsuspension.
pub const CONTRACT_USER_MODERATION_UNSUSPEND: u8 = 3;
/// The `action` byte of a warning.
pub const CONTRACT_USER_MODERATION_WARN: u8 = 4;
/// The `action` byte of a clearing of warnings.
pub const CONTRACT_USER_MODERATION_CLEAR_WARNINGS: u8 = 5;

/// The moderation `value` names; any other byte is refused.
pub(crate) fn user_moderation_kind(
    value: u8,
) -> Result<ContractUserModerationKind, PlatformWalletFFIResult> {
    match value {
        CONTRACT_USER_MODERATION_BAN => Ok(ContractUserModerationKind::Ban),
        CONTRACT_USER_MODERATION_UNBAN => Ok(ContractUserModerationKind::Unban),
        CONTRACT_USER_MODERATION_SUSPEND => Ok(ContractUserModerationKind::Suspend),
        CONTRACT_USER_MODERATION_UNSUSPEND => Ok(ContractUserModerationKind::Unsuspend),
        CONTRACT_USER_MODERATION_WARN => Ok(ContractUserModerationKind::Warn),
        CONTRACT_USER_MODERATION_CLEAR_WARNINGS => Ok(ContractUserModerationKind::ClearWarnings),
        other => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("action must be 0 to 5 (ban to clear warnings), not {other}"),
        )),
    }
}

/// The name a moderation of an identity is written with.
fn user_moderation_name(kind: ContractUserModerationKind) -> &'static str {
    match kind {
        ContractUserModerationKind::Ban => "ban",
        ContractUserModerationKind::Unban => "unban",
        ContractUserModerationKind::Suspend => "suspend",
        ContractUserModerationKind::Unsuspend => "unsuspend",
        ContractUserModerationKind::Warn => "warn",
        ContractUserModerationKind::ClearWarnings => "clearWarnings",
    }
}

/// The name a moderator's role is written with.
fn role_name(role: ContractModeratorRole) -> &'static str {
    match role {
        ContractModeratorRole::Owner => "owner",
        ContractModeratorRole::AppointedModerator => "appointedModerator",
        ContractModeratorRole::InterimModerator => "interimModerator",
        ContractModeratorRole::TeamLeader => "teamLeader",
        ContractModeratorRole::TeamMember => "teamMember",
    }
}

/// The name a team action's status is written with, as the team action
/// reads name it.
fn team_action_status_name(status: GroupActionStatus) -> &'static str {
    match status {
        GroupActionStatus::ActionActive => "active",
        GroupActionStatus::ActionClosed => "closed",
    }
}

fn base58(id: &Identifier) -> String {
    id.to_string(Encoding::Base58)
}

/// A reason from its two nullable C arguments.
///
/// # Safety
/// `text` must be null or a NUL-terminated string, and `reason_document_id`
/// null or 32 readable bytes, for the duration of the call.
pub(crate) unsafe fn read_reason(
    text: *const c_char,
    reason_document_id: *const u8,
) -> Result<ContractModerationReason, PlatformWalletFFIResult> {
    let text = if text.is_null() {
        String::new()
    } else {
        CStr::from_ptr(text).to_str()?.to_string()
    };
    let reason = ContractModerationReason::from_text(text);
    if reason_document_id.is_null() {
        Ok(reason)
    } else {
        Ok(reason.with_reason_document(read_identifier(reason_document_id)?))
    }
}

/// A NUL-terminated UTF-8 string argument.
///
/// # Safety
/// `value` must be a non-null NUL-terminated string for the duration of the
/// call.
unsafe fn read_str(value: *const c_char) -> Result<String, PlatformWalletFFIResult> {
    Ok(CStr::from_ptr(value).to_str()?.to_string())
}

fn settled_deletion_json(rule: &SettledDeletionRule) -> JsonValue {
    json!({
        "approvals": rule.approvals,
        "leader": rule.leader,
        "approversPredateDocument": rule.approvers_predate_document,
    })
}

fn document_type_offer_json(name: &str, offer: &DocumentTypeModerationOffer) -> JsonValue {
    let mut object = Map::new();
    object.insert("documentTypeName".to_string(), json!(name));
    object.insert("delete".to_string(), json!(offer.delete));
    if let Some(seconds) = offer.delete_within_seconds {
        object.insert("deleteWithinSeconds".to_string(), json!(seconds));
    }
    object.insert("restore".to_string(), json!(offer.restore));
    object.insert(
        "changeableFields".to_string(),
        json!(offer.changeable_fields),
    );
    if let Some(rule) = &offer.settled_deletion {
        object.insert("settledDeletion".to_string(), settled_deletion_json(rule));
    }
    JsonValue::Object(object)
}

/// Who of the wallet may moderate and what each may do, identities in id
/// order, user actions in list order and document types by name.
pub(crate) fn contract_moderation_offer_json(offer: &ContractModerationOffer) -> JsonValue {
    let identities: Vec<JsonValue> = offer
        .identities
        .iter()
        .map(|identity| {
            let user_actions: Vec<&str> = identity
                .user_actions
                .iter()
                .copied()
                .map(user_moderation_name)
                .collect();
            let document_types: Vec<JsonValue> = identity
                .document_types
                .iter()
                .map(|(name, offer)| document_type_offer_json(name, offer))
                .collect();
            json!({
                "identityId": base58(&identity.identity_id),
                "role": role_name(identity.role),
                "userActions": user_actions,
                "documentTypes": document_types,
                "approveTeamActions": identity.approve_team_actions,
            })
        })
        .collect();
    json!({
        "contractId": base58(&offer.contract_id),
        "moderatorsTeam": moderators_pot_team_name(offer.moderators),
        "identities": identities,
    })
}

/// The target's status after a moderation, in the status read's shape.
pub(crate) fn moderated_user_status_json(status: &ModeratedUserStatus) -> JsonValue {
    json!({
        "contractId": base58(&status.contract_id),
        "identityId": base58(&status.identity_id),
        "status": contract_moderation_status_json(&status.status),
    })
}

/// A kept value that does not serialize, as the wallet's error.
fn removal_json_error(error: serde_json::Error) -> PlatformWalletError {
    PlatformWalletError::InvalidIdentityData(format!("cannot write the removal record: {error}"))
}

/// What a deletion left: the record, in one removals-page entry's shape,
/// when the type keeps one, and the document as it was (base64) when it was
/// read before the deletion.
pub(crate) fn document_deletion_json(
    contract_id: &Identifier,
    document_type_name: &str,
    deletion: &ContractDocumentDeletion,
) -> Result<JsonValue, PlatformWalletError> {
    let mut object = Map::new();
    object.insert("contractId".to_string(), json!(base58(contract_id)));
    object.insert("documentTypeName".to_string(), json!(document_type_name));
    object.insert(
        "documentId".to_string(),
        json!(base58(&deletion.document_id)),
    );
    if let Some((removal, kept_values)) = &deletion.removal {
        object.insert(
            "removal".to_string(),
            contract_document_removal_json(deletion.document_id, removal, kept_values)
                .map_err(removal_json_error)?,
        );
    }
    if let Some(bytes) = &deletion.document_bytes {
        object.insert(
            "documentBytes".to_string(),
            json!(encode(bytes, Encoding::Base64)),
        );
    }
    Ok(JsonValue::Object(object))
}

/// The record a restore marked, in one removals-page entry's shape.
pub(crate) fn document_restore_json(
    contract_id: &Identifier,
    document_type_name: &str,
    restore: &ContractDocumentRestore,
) -> Result<JsonValue, PlatformWalletError> {
    Ok(json!({
        "contractId": base58(contract_id),
        "documentTypeName": document_type_name,
        "documentId": base58(&restore.document_id),
        "removal": contract_document_removal_json(
            restore.document_id,
            &restore.removal,
            &restore.kept_values,
        )
        .map_err(removal_json_error)?,
    }))
}

/// The document a field change left, in the document create's canonical
/// JSON (`confirmed_document_to_json`).
pub(crate) fn document_fields_change_json(
    contract_id: &Identifier,
    document_type_name: &str,
    document: &Document,
) -> Result<JsonValue, PlatformWalletError> {
    let document: JsonValue = serde_json::from_str(&confirmed_document_to_json(document)?)
        .map_err(|error| {
            PlatformWalletError::InvalidIdentityData(format!(
                "cannot write the changed document: {error}"
            ))
        })?;
    Ok(json!({
        "contractId": base58(contract_id),
        "documentTypeName": document_type_name,
        "document": document,
    }))
}

/// A team action after a proposal or an approval.
pub(crate) fn team_action_signature_json(
    contract_id: &Identifier,
    action_id: &Identifier,
    status: GroupActionStatus,
) -> JsonValue {
    json!({
        "contractId": base58(contract_id),
        "actionId": base58(action_id),
        "status": team_action_status_name(status),
    })
}

/// What a host should know after the offer read panicked: a read signs and sends nothing.
const OFFER_PANIC_GUIDANCE: &str = "Nothing was signed or broadcast; ask again.";

/// What a host should know after a moderation write panicked: the panic can strike after the
/// transition was broadcast, so whether it took effect is unknown until the contract's
/// moderation state is read again.
const WRITE_PANIC_GUIDANCE: &str = "The moderation may or may not have been broadcast; read \
     the contract's moderation state again before sending it once more.";

/// Write `json` to `*out_json`, handing its ownership to the caller.
///
/// # Safety
/// `out_json` must be a valid, writable pointer.
unsafe fn write_json(
    json: JsonValue,
    out_json: *mut *mut c_char,
) -> Result<(), PlatformWalletFFIResult> {
    // A NUL byte inside the JSON would be a serializer bug; refuse it
    // rather than panic across the FFI boundary.
    *out_json = CString::new(json.to_string())?.into_raw();
    Ok(())
}

/// Which of this wallet's signing identities may moderate `contract_id`
/// and what each may do, read through
/// `IdentityWallet::contract_moderation_offer` on the platform-wallet 8 MB
/// worker stack.
///
/// On success writes to `*out_json`:
///
/// ```json
/// {"contractId": "<base58>",
///  "moderatorsTeam": "noModeration" | "declared" | "electedInterim" | "electedSeated",
///  "identities": [{"identityId": "<base58>",
///                  "role": "owner" | "appointedModerator" | "interimModerator"
///                          | "teamLeader" | "teamMember",
///                  "userActions": ["ban", "unban", "suspend", "unsuspend", "warn",
///                                  "clearWarnings"],
///                  "documentTypes": [{"documentTypeName": "post", "delete": true,
///                                     "deleteWithinSeconds": 86400, "restore": true,
///                                     "changeableFields": ["flag"],
///                                     "settledDeletion": {"approvals": 2, "leader": true,
///                                                         "approversPredateDocument": true}}],
///                  "approveTeamActions": false}]}
/// ```
///
/// `identities` lists only the wallet's identities that moderate the
/// contract; `documentTypes` only the types on which an identity may do
/// something. `deleteWithinSeconds` is absent when the type has no window,
/// `settledDeletion` when the identity may propose no settled deletion.
///
/// It names identities by role, not by key: an identity named here whose
/// signer holds no CRITICAL authentication key without contract bounds is
/// still named, and its moderation fails when sent with that error, which
/// the host shows.
/// Release the string with `platform_wallet_string_free`. On any error
/// `*out_json` is left null.
///
/// # Errors
/// - `NotFound` when Platform holds no contract `contract_id`;
/// - a failed read carries the SDK's error.
///
/// # Safety
/// - `contract_id` must point to 32 readable bytes.
/// - `out_json` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_contract_moderation_offer(
    wallet_handle: Handle,
    contract_id: *const u8,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // `block_on_worker` re-raises a worker's panic here, inside the
    // `extern "C"` frame, where it would abort the host.
    catch_panic_to_code(
        "platform_wallet_contract_moderation_offer",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        OFFER_PANIC_GUIDANCE,
        || unsafe { contract_moderation_offer_inner(wallet_handle, contract_id, out_json) },
    )
}

/// The body of [`platform_wallet_contract_moderation_offer`], run inside its panic
/// guard.
///
/// # Safety
/// As [`platform_wallet_contract_moderation_offer`].
unsafe fn contract_moderation_offer_inner(
    wallet_handle: Handle,
    contract_id: *const u8,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // The out slot is checked and nulled before anything else can
    // return, so every error leaves it null.
    check_ptr!(out_json);
    *out_json = ptr::null_mut();

    let contract_id = unwrap_result_or_return!(read_identifier(contract_id));

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let offer = identity_wallet
                .contract_moderation_offer(&contract_id)
                .await?;
            Ok::<_, PlatformWalletError>(contract_moderation_offer_json(&offer))
        })
    });
    let result = unwrap_option_or_return!(option);
    let json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_json(json, out_json));
    PlatformWalletFFIResult::ok()
}

/// Send one moderation `action` of `target_identity_id` on `contract_id`,
/// signed by `moderator_identity_id`, an identity this wallet holds, via the
/// external `signer_handle`, through
/// `IdentityWallet::moderate_contract_user_with_signer`.
///
/// `action` is 0 ban, 1 unban, 2 suspend, 3 unsuspend, 4 warn, 5 clear
/// warnings. `suspend_until_ms` points to the block time a suspension lasts
/// until, in milliseconds, and is null for every other action (ignored
/// there). The reason (see the module documentation) is stored with a ban,
/// a suspension and a warning.
///
/// On success writes to `*out_json` the target's status on the list the
/// moderation edited, as its proof shows it (a ban proves every barring
/// list the contract keeps): `{"contractId", "identityId", "status": {...}}`,
/// `status` in the shape of `dash_sdk_contract_moderation_get_status`.
/// Release it with `platform_wallet_string_free`. On any error `*out_json`
/// is left null.
///
/// # Errors
/// - `ErrorInvalidParameter` for an `action` above 5, or a suspension with
///   a null `suspend_until_ms`;
/// - `ErrorIdentityNotFound` when the wallet holds no signing identity
///   `moderator_identity_id`;
/// - a refusal by Platform carries its `consensus_code`: a list the
///   contract does not keep, a signer that does not moderate it, a
///   protected or missing target, an identity already banned (41103), not
///   banned, not suspended, the most warnings reached, a seated team's
///   unlisted reason.
///
/// # Safety
/// - `moderator_identity_id`, `contract_id` and `target_identity_id` must
///   each point to 32 readable bytes.
/// - `suspend_until_ms` must be null or point to a readable `u64`.
/// - `reason_text` must be null or a NUL-terminated string, and
///   `reason_document_id` null or 32 readable bytes.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_json` must be a valid, writable pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_contract_user_moderate(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    target_identity_id: *const u8,
    action: u8,
    suspend_until_ms: *const u64,
    reason_text: *const c_char,
    reason_document_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // `block_on_worker` re-raises a worker's panic here, inside the
    // `extern "C"` frame, where it would abort the host.
    catch_panic_to_code(
        "platform_wallet_contract_user_moderate",
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        WRITE_PANIC_GUIDANCE,
        || unsafe {
            contract_user_moderate_inner(
                wallet_handle,
                moderator_identity_id,
                contract_id,
                target_identity_id,
                action,
                suspend_until_ms,
                reason_text,
                reason_document_id,
                signer_handle,
                out_json,
            )
        },
    )
}

/// The body of [`platform_wallet_contract_user_moderate`], run inside its panic
/// guard.
///
/// # Safety
/// As [`platform_wallet_contract_user_moderate`].
#[allow(clippy::too_many_arguments)]
unsafe fn contract_user_moderate_inner(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    target_identity_id: *const u8,
    action: u8,
    suspend_until_ms: *const u64,
    reason_text: *const c_char,
    reason_document_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // The out slot is checked and nulled before anything else can
    // return, so every error leaves it null.
    check_ptr!(out_json);
    *out_json = ptr::null_mut();
    check_ptr!(signer_handle);

    let moderator_id = unwrap_result_or_return!(read_identifier(moderator_identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(contract_id));
    let target_id = unwrap_result_or_return!(read_identifier(target_identity_id));
    let kind = unwrap_result_or_return!(user_moderation_kind(action));
    let suspend_until = if suspend_until_ms.is_null() {
        None
    } else {
        Some(*suspend_until_ms)
    };
    let reason = unwrap_result_or_return!(read_reason(reason_text, reason_document_id));
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let status = identity_wallet
                .moderate_contract_user_with_signer(
                    &moderator_id,
                    &contract_id,
                    &target_id,
                    kind,
                    suspend_until,
                    reason,
                    signer,
                )
                .await?;
            Ok::<_, PlatformWalletError>(moderated_user_status_json(&status))
        })
    });
    let result = unwrap_option_or_return!(option);
    let json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_json(json, out_json));
    PlatformWalletFFIResult::ok()
}

/// Delete document `document_id` of `document_type_name` on `contract_id`
/// as a moderator, signed by `moderator_identity_id`, an identity this
/// wallet holds, via the external `signer_handle`, through
/// `IdentityWallet::delete_contract_document_with_signer`. The reason (see
/// the module documentation) is stored in the removal record.
///
/// On success writes to `*out_json`
/// `{"contractId", "documentTypeName", "documentId", "removal": {...},
/// "documentBytes": "<base64>"}`: `removal` is the record the deletion left,
/// in the shape of one entry of `dash_sdk_contract_moderation_get_document_removals`,
/// absent on a type whose deletions keep no record; `documentBytes` is the
/// document as it was just before the deletion, serialized under its type,
/// which `platform_wallet_contract_document_restore` takes back, absent
/// when the document could not be read first. Release the string with
/// `platform_wallet_string_free`. On any error `*out_json` is left null.
///
/// # Errors
/// - `NotFound` when Platform holds no contract `contract_id`;
/// - `ErrorInvalidParameter` when the contract has no such type;
/// - `ErrorIdentityNotFound` when the wallet holds no signing identity
///   `moderator_identity_id`;
/// - a refusal by Platform carries its `consensus_code`: a type moderators
///   may not delete from, a signer that does not moderate the contract, a
///   protected owner, a document that does not exist, one past the type's
///   window.
///
/// # Safety
/// - `moderator_identity_id`, `contract_id` and `document_id` must each
///   point to 32 readable bytes; `document_type_name` must be a
///   NUL-terminated string.
/// - `reason_text` must be null or a NUL-terminated string, and
///   `reason_document_id` null or 32 readable bytes.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_json` must be a valid, writable pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_contract_document_delete(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    reason_text: *const c_char,
    reason_document_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // `block_on_worker` re-raises a worker's panic here, inside the
    // `extern "C"` frame, where it would abort the host.
    catch_panic_to_code(
        "platform_wallet_contract_document_delete",
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        WRITE_PANIC_GUIDANCE,
        || unsafe {
            contract_document_delete_inner(
                wallet_handle,
                moderator_identity_id,
                contract_id,
                document_type_name,
                document_id,
                reason_text,
                reason_document_id,
                signer_handle,
                out_json,
            )
        },
    )
}

/// The body of [`platform_wallet_contract_document_delete`], run inside its panic
/// guard.
///
/// # Safety
/// As [`platform_wallet_contract_document_delete`].
#[allow(clippy::too_many_arguments)]
unsafe fn contract_document_delete_inner(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    reason_text: *const c_char,
    reason_document_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // The out slot is checked and nulled before anything else can
    // return, so every error leaves it null.
    check_ptr!(out_json);
    *out_json = ptr::null_mut();
    check_ptr!(document_type_name);
    check_ptr!(signer_handle);

    let moderator_id = unwrap_result_or_return!(read_identifier(moderator_identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(contract_id));
    let document_type_name = unwrap_result_or_return!(read_str(document_type_name));
    let document_id = unwrap_result_or_return!(read_identifier(document_id));
    let reason = unwrap_result_or_return!(read_reason(reason_text, reason_document_id));
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let deletion = identity_wallet
                .delete_contract_document_with_signer(
                    &moderator_id,
                    &contract_id,
                    &document_type_name,
                    &document_id,
                    reason,
                    signer,
                )
                .await?;
            document_deletion_json(&contract_id, &document_type_name, &deletion)
        })
    });
    let result = unwrap_option_or_return!(option);
    let json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_json(json, out_json));
    PlatformWalletFFIResult::ok()
}

/// Restore a document of `document_type_name` on `contract_id` that a
/// moderator deleted, signed by `moderator_identity_id`, an identity this
/// wallet holds, via the external `signer_handle`, through
/// `IdentityWallet::restore_contract_document_with_signer`.
/// `document_bytes` is the document as it was, serialized under its type:
/// the decoded `documentBytes` of `platform_wallet_contract_document_delete`.
///
/// On success writes to `*out_json` `{"contractId", "documentTypeName",
/// "documentId", "removal": {...}}`, `removal` the record now marked
/// restored, in the removals read's entry shape. Release the string with
/// `platform_wallet_string_free`. On any error `*out_json` is left null.
///
/// # Errors
/// - `ErrorInvalidParameter` for zero bytes, a contract without such a
///   type, or bytes that are not a document of it;
/// - `NotFound` when Platform holds no contract `contract_id`;
/// - `ErrorIdentityNotFound` when the wallet holds no signing identity
///   `moderator_identity_id`;
/// - a refusal by Platform carries its `consensus_code`: no removal record
///   (41119), past the week (41120), not the document deleted (41121),
///   already restored (41122), a deletion the seated team approved together
///   (41209), a unique value taken meanwhile.
///
/// # Safety
/// - `moderator_identity_id` and `contract_id` must each point to 32
///   readable bytes; `document_type_name` must be a NUL-terminated string.
/// - `document_bytes` must point to `document_bytes_len` readable bytes.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_json` must be a valid, writable pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_contract_document_restore(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_bytes: *const u8,
    document_bytes_len: usize,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // `block_on_worker` re-raises a worker's panic here, inside the
    // `extern "C"` frame, where it would abort the host.
    catch_panic_to_code(
        "platform_wallet_contract_document_restore",
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        WRITE_PANIC_GUIDANCE,
        || unsafe {
            contract_document_restore_inner(
                wallet_handle,
                moderator_identity_id,
                contract_id,
                document_type_name,
                document_bytes,
                document_bytes_len,
                signer_handle,
                out_json,
            )
        },
    )
}

/// The body of [`platform_wallet_contract_document_restore`], run inside its panic
/// guard.
///
/// # Safety
/// As [`platform_wallet_contract_document_restore`].
#[allow(clippy::too_many_arguments)]
unsafe fn contract_document_restore_inner(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_bytes: *const u8,
    document_bytes_len: usize,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // The out slot is checked and nulled before anything else can
    // return, so every error leaves it null.
    check_ptr!(out_json);
    *out_json = ptr::null_mut();
    check_ptr!(document_type_name);
    check_ptr!(document_bytes);
    check_ptr!(signer_handle);

    if document_bytes_len == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "document_bytes is empty",
        );
    }
    let moderator_id = unwrap_result_or_return!(read_identifier(moderator_identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(contract_id));
    let document_type_name = unwrap_result_or_return!(read_str(document_type_name));
    let document_bytes = slice::from_raw_parts(document_bytes, document_bytes_len).to_vec();
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let restore = identity_wallet
                .restore_contract_document_with_signer(
                    &moderator_id,
                    &contract_id,
                    &document_type_name,
                    &document_bytes,
                    signer,
                )
                .await?;
            document_restore_json(&contract_id, &document_type_name, &restore)
        })
    });
    let result = unwrap_option_or_return!(option);
    let json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_json(json, out_json));
    PlatformWalletFFIResult::ok()
}

/// Set the moderator fields `fields_json` names on document `document_id`
/// of `document_type_name` on `contract_id`, signed by
/// `moderator_identity_id`, an identity this wallet holds, via the external
/// `signer_handle`, through
/// `IdentityWallet::change_contract_document_fields_with_signer`.
///
/// `fields_json` is a NUL-terminated JSON object keyed by top-level property
/// name, a `null` value removing the field; identifiers as base58 and byte
/// arrays as base64 or hex strings, read under the contract's document type
/// as a document create reads its properties. The reason (see the module
/// documentation) is the change's.
///
/// On success writes to `*out_json` `{"contractId", "documentTypeName",
/// "document": {...}}`, `document` the document as it now stands, proved, in
/// the canonical JSON `platform_wallet_create_document_with_signer` writes
/// (`$id`, `$ownerId` and `$revision` among its fields). Release the string
/// with `platform_wallet_string_free`. On any error `*out_json` is left null.
///
/// # Errors
/// - `ErrorInvalidParameter` for `fields_json` that is not a JSON object,
///   or a contract without such a type;
/// - `NotFound` when Platform holds no contract `contract_id`;
/// - `ErrorIdentityNotFound` when the wallet holds no signing identity
///   `moderator_identity_id`;
/// - a refusal by Platform carries its `consensus_code`: a field the type
///   does not keep for its moderators (41123), a signer that does not
///   moderate the contract (41101), a seated team without
///   `changeDocumentFields` (41201), a document left outside its schema.
///
/// # Safety
/// - `moderator_identity_id`, `contract_id` and `document_id` must each
///   point to 32 readable bytes; `document_type_name` and `fields_json` must
///   be NUL-terminated strings.
/// - `reason_text` must be null or a NUL-terminated string, and
///   `reason_document_id` null or 32 readable bytes.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_json` must be a valid, writable pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_contract_document_change_fields(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    fields_json: *const c_char,
    reason_text: *const c_char,
    reason_document_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // `block_on_worker` re-raises a worker's panic here, inside the
    // `extern "C"` frame, where it would abort the host.
    catch_panic_to_code(
        "platform_wallet_contract_document_change_fields",
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        WRITE_PANIC_GUIDANCE,
        || unsafe {
            contract_document_change_fields_inner(
                wallet_handle,
                moderator_identity_id,
                contract_id,
                document_type_name,
                document_id,
                fields_json,
                reason_text,
                reason_document_id,
                signer_handle,
                out_json,
            )
        },
    )
}

/// The body of [`platform_wallet_contract_document_change_fields`], run inside its panic
/// guard.
///
/// # Safety
/// As [`platform_wallet_contract_document_change_fields`].
#[allow(clippy::too_many_arguments)]
unsafe fn contract_document_change_fields_inner(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    fields_json: *const c_char,
    reason_text: *const c_char,
    reason_document_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // The out slot is checked and nulled before anything else can
    // return, so every error leaves it null.
    check_ptr!(out_json);
    *out_json = ptr::null_mut();
    check_ptr!(document_type_name);
    check_ptr!(fields_json);
    check_ptr!(signer_handle);

    let moderator_id = unwrap_result_or_return!(read_identifier(moderator_identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(contract_id));
    let document_type_name = unwrap_result_or_return!(read_str(document_type_name));
    let document_id = unwrap_result_or_return!(read_identifier(document_id));
    let fields_json = unwrap_result_or_return!(read_str(fields_json));
    let reason = unwrap_result_or_return!(read_reason(reason_text, reason_document_id));
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let document = identity_wallet
                .change_contract_document_fields_with_signer(
                    &moderator_id,
                    &contract_id,
                    &document_type_name,
                    &document_id,
                    &fields_json,
                    reason,
                    signer,
                )
                .await?;
            document_fields_change_json(&contract_id, &document_type_name, &document)
        })
    });
    let result = unwrap_option_or_return!(option);
    let json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_json(json, out_json));
    PlatformWalletFFIResult::ok()
}

/// Propose, as a member of the seated team of `contract_id`, the deletion
/// of settled document `document_id` of `document_type_name`, signed by
/// `moderator_identity_id`, an identity this wallet holds, via the external
/// `signer_handle`, through
/// `IdentityWallet::propose_settled_contract_document_deletion_with_signer`.
/// The reason (see the module documentation) must name a reason document
/// the team's proposal lists.
///
/// On success writes to `*out_json` `{"contractId", "actionId", "status":
/// "active" | "closed"}`: the team action the proposal opened, which the
/// other members approve by `actionId`, `"closed"` once the document is
/// gone. Release the string with `platform_wallet_string_free`. On any
/// error `*out_json` is left null.
///
/// # Errors
/// - `ErrorIdentityNotFound` when the wallet holds no signing identity
///   `moderator_identity_id`;
/// - a refusal by Platform carries its `consensus_code`: a type without
///   `deleteSettled`, no seated team, a signer not on it, an unlisted
///   reason, a document not settled yet, a member added after the document.
///
/// # Safety
/// - `moderator_identity_id`, `contract_id` and `document_id` must each
///   point to 32 readable bytes; `document_type_name` must be a
///   NUL-terminated string.
/// - `reason_text` must be null or a NUL-terminated string, and
///   `reason_document_id` null or 32 readable bytes.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_json` must be a valid, writable pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_contract_document_propose_settled_deletion(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    reason_text: *const c_char,
    reason_document_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // `block_on_worker` re-raises a worker's panic here, inside the
    // `extern "C"` frame, where it would abort the host.
    catch_panic_to_code(
        "platform_wallet_contract_document_propose_settled_deletion",
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        WRITE_PANIC_GUIDANCE,
        || unsafe {
            contract_document_propose_settled_deletion_inner(
                wallet_handle,
                moderator_identity_id,
                contract_id,
                document_type_name,
                document_id,
                reason_text,
                reason_document_id,
                signer_handle,
                out_json,
            )
        },
    )
}

/// The body of [`platform_wallet_contract_document_propose_settled_deletion`], run inside its panic
/// guard.
///
/// # Safety
/// As [`platform_wallet_contract_document_propose_settled_deletion`].
#[allow(clippy::too_many_arguments)]
unsafe fn contract_document_propose_settled_deletion_inner(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    reason_text: *const c_char,
    reason_document_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // The out slot is checked and nulled before anything else can
    // return, so every error leaves it null.
    check_ptr!(out_json);
    *out_json = ptr::null_mut();
    check_ptr!(document_type_name);
    check_ptr!(signer_handle);

    let moderator_id = unwrap_result_or_return!(read_identifier(moderator_identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(contract_id));
    let document_type_name = unwrap_result_or_return!(read_str(document_type_name));
    let document_id = unwrap_result_or_return!(read_identifier(document_id));
    let reason = unwrap_result_or_return!(read_reason(reason_text, reason_document_id));
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let (action_id, status) = identity_wallet
                .propose_settled_contract_document_deletion_with_signer(
                    &moderator_id,
                    &contract_id,
                    &document_type_name,
                    &document_id,
                    reason,
                    signer,
                )
                .await?;
            Ok::<_, PlatformWalletError>(team_action_signature_json(
                &contract_id,
                &action_id,
                status,
            ))
        })
    });
    let result = unwrap_option_or_return!(option);
    let json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_json(json, out_json));
    PlatformWalletFFIResult::ok()
}

/// Approve, as a member of the seated team of `contract_id`, team action
/// `action_id` another member proposed, signed by `moderator_identity_id`,
/// an identity this wallet holds, via the external `signer_handle`, through
/// `IdentityWallet::approve_contract_team_action_with_signer`.
///
/// On success writes to `*out_json` `{"contractId", "actionId", "status":
/// "active" | "closed"}`: `"active"` while the approvals still fall short of
/// the rule, `"closed"` once the action ran. Release the string with
/// `platform_wallet_string_free`. On any error `*out_json` is left null.
///
/// # Errors
/// - `ErrorIdentityNotFound` when the wallet holds no signing identity
///   `moderator_identity_id`;
/// - a refusal by Platform carries its `consensus_code`: no such action,
///   one this member already approved, one already closed, a document
///   changed since the proposal, a member added after the document.
///
/// # Safety
/// - `moderator_identity_id`, `contract_id` and `action_id` must each point
///   to 32 readable bytes.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_json` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_contract_team_action_approve(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    action_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // `block_on_worker` re-raises a worker's panic here, inside the
    // `extern "C"` frame, where it would abort the host.
    catch_panic_to_code(
        "platform_wallet_contract_team_action_approve",
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        WRITE_PANIC_GUIDANCE,
        || unsafe {
            contract_team_action_approve_inner(
                wallet_handle,
                moderator_identity_id,
                contract_id,
                action_id,
                signer_handle,
                out_json,
            )
        },
    )
}

/// The body of [`platform_wallet_contract_team_action_approve`], run inside its panic
/// guard.
///
/// # Safety
/// As [`platform_wallet_contract_team_action_approve`].
unsafe fn contract_team_action_approve_inner(
    wallet_handle: Handle,
    moderator_identity_id: *const u8,
    contract_id: *const u8,
    action_id: *const u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // The out slot is checked and nulled before anything else can
    // return, so every error leaves it null.
    check_ptr!(out_json);
    *out_json = ptr::null_mut();
    check_ptr!(signer_handle);

    let moderator_id = unwrap_result_or_return!(read_identifier(moderator_identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(contract_id));
    let action_id = unwrap_result_or_return!(read_identifier(action_id));
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let status = identity_wallet
                .approve_contract_team_action_with_signer(
                    &moderator_id,
                    &contract_id,
                    &action_id,
                    signer,
                )
                .await?;
            Ok::<_, PlatformWalletError>(team_action_signature_json(
                &contract_id,
                &action_id,
                status,
            ))
        })
    });
    let result = unwrap_option_or_return!(option);
    let json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_json(json, out_json));
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::error::StateTransitionBroadcastError;
    use dpp::consensus::state::contract_moderation::ContractUserAlreadyBannedError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::config::moderation::{
        ContractBan, ContractDocumentRemoval, ContractModerationListStatus,
        ContractModerationListStatuses,
    };
    use dpp::document::DocumentV0;
    use dpp::platform_value::Value;
    use platform_wallet::{IdentityModerationOffer, ModeratorsPotTeam};
    use std::collections::{BTreeMap, BTreeSet};
    use std::ffi::CString;

    fn id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    #[test]
    fn should_read_the_action_byte_and_refuse_any_other() {
        for (byte, kind) in [
            (0, ContractUserModerationKind::Ban),
            (1, ContractUserModerationKind::Unban),
            (2, ContractUserModerationKind::Suspend),
            (3, ContractUserModerationKind::Unsuspend),
            (4, ContractUserModerationKind::Warn),
            (5, ContractUserModerationKind::ClearWarnings),
        ] {
            assert_eq!(user_moderation_kind(byte).expect("a moderation"), kind);
        }
        for byte in [6, 9, u8::MAX] {
            let refused = user_moderation_kind(byte).expect_err("not a moderation");
            assert_eq!(
                refused.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
        }
    }

    #[test]
    fn should_read_a_reason_from_its_nullable_parts() {
        unsafe {
            assert_eq!(
                read_reason(ptr::null(), ptr::null()).expect("no reason"),
                ContractModerationReason::default()
            );
            let text = CString::new("spam").expect("text");
            let reason_document = [7u8; 32];
            assert_eq!(
                read_reason(text.as_ptr(), reason_document.as_ptr()).expect("a reason"),
                ContractModerationReason::from_text("spam").with_reason_document(id(7))
            );
            let not_utf8 = [0xFFu8, 0];
            assert!(read_reason(not_utf8.as_ptr() as *const c_char, ptr::null()).is_err());
        }
    }

    #[test]
    fn should_write_who_may_moderate_and_what_each_may_do() {
        let offer = ContractModerationOffer {
            contract_id: id(1),
            moderators: ModeratorsPotTeam::ElectedSeated,
            identities: vec![IdentityModerationOffer {
                identity_id: id(5),
                role: ContractModeratorRole::TeamLeader,
                user_actions: BTreeSet::from([
                    ContractUserModerationKind::Unban,
                    ContractUserModerationKind::Ban,
                ]),
                document_types: BTreeMap::from([(
                    "post".to_string(),
                    DocumentTypeModerationOffer {
                        delete: true,
                        delete_within_seconds: Some(86400),
                        restore: false,
                        changeable_fields: BTreeSet::from(["flag".to_string()]),
                        settled_deletion: Some(SettledDeletionRule {
                            leader: true,
                            approvals: 2,
                            approvers_predate_document: true,
                        }),
                    },
                )]),
                approve_team_actions: true,
            }],
        };
        assert_eq!(
            contract_moderation_offer_json(&offer),
            json!({
                "contractId": base58(&id(1)),
                "moderatorsTeam": "electedSeated",
                "identities": [{
                    "identityId": base58(&id(5)),
                    "role": "teamLeader",
                    "userActions": ["ban", "unban"],
                    "documentTypes": [{
                        "documentTypeName": "post",
                        "delete": true,
                        "deleteWithinSeconds": 86400,
                        "restore": false,
                        "changeableFields": ["flag"],
                        "settledDeletion": {
                            "approvals": 2,
                            "leader": true,
                            "approversPredateDocument": true,
                        },
                    }],
                    "approveTeamActions": true,
                }],
            })
        );
        let no_window = document_type_offer_json(
            "note",
            &DocumentTypeModerationOffer {
                delete: true,
                delete_within_seconds: None,
                restore: true,
                changeable_fields: BTreeSet::new(),
                settled_deletion: None,
            },
        );
        assert!(no_window.get("deleteWithinSeconds").is_none());
        assert!(no_window.get("settledDeletion").is_none());
        for (kind, name) in [
            (ContractUserModerationKind::Suspend, "suspend"),
            (ContractUserModerationKind::Unsuspend, "unsuspend"),
            (ContractUserModerationKind::Warn, "warn"),
            (ContractUserModerationKind::ClearWarnings, "clearWarnings"),
        ] {
            assert_eq!(user_moderation_name(kind), name);
        }
        for (role, name) in [
            (ContractModeratorRole::Owner, "owner"),
            (
                ContractModeratorRole::AppointedModerator,
                "appointedModerator",
            ),
            (ContractModeratorRole::InterimModerator, "interimModerator"),
            (ContractModeratorRole::TeamMember, "teamMember"),
        ] {
            assert_eq!(role_name(role), name);
        }
    }

    /// The status a moderation proves is written as the status read writes it.
    #[test]
    fn should_write_the_moderated_status_in_the_status_read_shape() {
        let statuses =
            ContractModerationListStatuses(vec![ContractModerationListStatus::Banlist {
                ban: Some(ContractBan {
                    reason: ContractModerationReason::from_text("spam"),
                }),
            }]);
        let json = moderated_user_status_json(&ModeratedUserStatus {
            contract_id: id(1),
            identity_id: id(2),
            status: statuses.clone(),
        });
        assert_eq!(json["contractId"], base58(&id(1)));
        assert_eq!(json["identityId"], base58(&id(2)));
        assert_eq!(json["status"], contract_moderation_status_json(&statuses));
        assert_eq!(json["status"]["banned"], true);
    }

    fn removal() -> ContractDocumentRemoval {
        ContractDocumentRemoval {
            document_owner_id: id(20),
            moderator_id: id(21),
            reason: ContractModerationReason::from_text("off topic"),
            removed_at: 1_700_000_000_000,
            document_hash: [0xab; 32],
            restoration: None,
            kept_fields: vec![],
        }
    }

    #[test]
    fn should_write_a_deletion_with_its_record_and_the_document_bytes() {
        let kept = BTreeMap::from([("hashtag".to_string(), Value::Text("dash".to_string()))]);
        let deletion = ContractDocumentDeletion {
            document_id: id(3),
            removal: Some((removal(), kept.clone())),
            document_bytes: Some(vec![1, 2, 3]),
        };
        let json = document_deletion_json(&id(1), "post", &deletion).expect("writes");
        assert_eq!(json["contractId"], base58(&id(1)));
        assert_eq!(json["documentTypeName"], "post");
        assert_eq!(json["documentId"], base58(&id(3)));
        assert_eq!(
            json["removal"],
            contract_document_removal_json(id(3), &removal(), &kept).expect("writes")
        );
        assert_eq!(json["documentBytes"], "AQID");

        let unrecorded = document_deletion_json(
            &id(1),
            "post",
            &ContractDocumentDeletion {
                document_id: id(3),
                removal: None,
                document_bytes: None,
            },
        )
        .expect("writes");
        assert!(unrecorded.get("removal").is_none());
        assert!(unrecorded.get("documentBytes").is_none());

        let restore = document_restore_json(
            &id(1),
            "post",
            &ContractDocumentRestore {
                document_id: id(3),
                removal: removal(),
                kept_values: BTreeMap::new(),
            },
        )
        .expect("writes");
        assert_eq!(restore["removal"]["documentId"], base58(&id(3)));
    }

    #[test]
    fn should_write_the_changed_document_and_a_team_action() {
        let document: Document = DocumentV0 {
            id: id(3),
            owner_id: id(4),
            revision: Some(2),
            ..Default::default()
        }
        .into();
        let json = document_fields_change_json(&id(1), "post", &document).expect("writes");
        assert_eq!(json["documentTypeName"], "post");
        assert_eq!(json["document"]["$id"], base58(&id(3)));
        assert_eq!(json["document"]["$revision"], 2);

        assert_eq!(
            team_action_signature_json(&id(1), &id(9), GroupActionStatus::ActionActive),
            json!({"contractId": base58(&id(1)), "actionId": base58(&id(9)), "status": "active"})
        );
        assert_eq!(
            team_action_signature_json(&id(1), &id(9), GroupActionStatus::ActionClosed)["status"],
            "closed"
        );
    }

    /// Every pointer and the action byte are checked before the wallet is
    /// looked up, and the JSON out-param is left null on the way out.
    #[test]
    fn should_refuse_null_pointers_and_a_bad_action_before_the_wallet_is_looked_up() {
        let id = [0u8; 32];
        let name = CString::new("post").expect("name");
        let fields = CString::new("{}").expect("fields");
        let bytes = [1u8; 4];
        let mut out_json: *mut c_char = ptr::null_mut();
        // A dangling but non-null signer: every call below returns before
        // the signer is read.
        let signer = ptr::NonNull::<SignerHandle>::dangling().as_ptr();
        unsafe {
            for result in [
                platform_wallet_contract_moderation_offer(0, id.as_ptr(), ptr::null_mut()),
                platform_wallet_contract_user_moderate(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    id.as_ptr(),
                    0,
                    ptr::null(),
                    ptr::null(),
                    ptr::null(),
                    ptr::null_mut(),
                    &mut out_json,
                ),
                platform_wallet_contract_document_delete(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    id.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    signer,
                    &mut out_json,
                ),
                platform_wallet_contract_document_restore(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    name.as_ptr(),
                    ptr::null(),
                    4,
                    signer,
                    &mut out_json,
                ),
                platform_wallet_contract_document_change_fields(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    name.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    ptr::null(),
                    signer,
                    &mut out_json,
                ),
                platform_wallet_contract_document_propose_settled_deletion(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    name.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    signer,
                    ptr::null_mut(),
                ),
                platform_wallet_contract_team_action_approve(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null_mut(),
                    &mut out_json,
                ),
            ] {
                assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            }
            for result in [
                platform_wallet_contract_moderation_offer(0, ptr::null(), &mut out_json),
                platform_wallet_contract_user_moderate(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    0,
                    ptr::null(),
                    ptr::null(),
                    ptr::null(),
                    signer,
                    &mut out_json,
                ),
                platform_wallet_contract_document_delete(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    name.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    ptr::null(),
                    signer,
                    &mut out_json,
                ),
                platform_wallet_contract_document_change_fields(
                    0,
                    ptr::null(),
                    id.as_ptr(),
                    name.as_ptr(),
                    id.as_ptr(),
                    fields.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    signer,
                    &mut out_json,
                ),
                platform_wallet_contract_team_action_approve(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    signer,
                    &mut out_json,
                ),
            ] {
                assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
            }
            let bad_action = platform_wallet_contract_user_moderate(
                0,
                id.as_ptr(),
                id.as_ptr(),
                id.as_ptr(),
                6,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                signer,
                &mut out_json,
            );
            assert_eq!(
                bad_action.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
            let empty_bytes = platform_wallet_contract_document_restore(
                0,
                id.as_ptr(),
                id.as_ptr(),
                name.as_ptr(),
                bytes.as_ptr(),
                0,
                signer,
                &mut out_json,
            );
            assert_eq!(
                empty_bytes.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
            assert!(out_json.is_null());
        }
    }

    /// The JSON out-param is nulled before any other pointer is checked, so
    /// a null signer or document type name leaves it null as every other
    /// error does.
    #[test]
    fn should_null_the_json_out_param_before_refusing_any_other_null_pointer() {
        let id = [0u8; 32];
        let name = CString::new("post").expect("name");
        let fields = CString::new("{}").expect("fields");
        let bytes = [1u8; 4];
        // A stale, non-null value each call must overwrite; never read.
        let stale = ptr::NonNull::<c_char>::dangling().as_ptr();
        let calls: [&dyn Fn(*mut *mut c_char) -> PlatformWalletFFIResult; 6] = [
            &|out| unsafe {
                platform_wallet_contract_user_moderate(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    id.as_ptr(),
                    0,
                    ptr::null(),
                    ptr::null(),
                    ptr::null(),
                    ptr::null_mut(),
                    out,
                )
            },
            &|out| unsafe {
                platform_wallet_contract_document_delete(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    id.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    ptr::null_mut(),
                    out,
                )
            },
            &|out| unsafe {
                platform_wallet_contract_document_restore(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    bytes.as_ptr(),
                    bytes.len(),
                    ptr::null_mut(),
                    out,
                )
            },
            &|out| unsafe {
                platform_wallet_contract_document_change_fields(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    name.as_ptr(),
                    id.as_ptr(),
                    fields.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    ptr::null_mut(),
                    out,
                )
            },
            &|out| unsafe {
                platform_wallet_contract_document_propose_settled_deletion(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null(),
                    id.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    ptr::null_mut(),
                    out,
                )
            },
            &|out| unsafe {
                platform_wallet_contract_team_action_approve(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    id.as_ptr(),
                    ptr::null_mut(),
                    out,
                )
            },
        ];
        for (index, call) in calls.iter().enumerate() {
            let mut out_json = stale;
            let result = call(&mut out_json);
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorNullPointer,
                "call {index}"
            );
            assert!(out_json.is_null(), "call {index} left the out-param set");
        }
    }

    /// Platform refuses a ban of an identity already banned; the host gets
    /// the refusal's consensus code beside the result code.
    #[test]
    fn should_carry_the_consensus_code_of_a_ban_of_a_banned_identity() {
        let cause = ConsensusError::from(ContractUserAlreadyBannedError::new(id(1), id(2)));
        let result: PlatformWalletFFIResult = PlatformWalletError::Sdk(
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 41103,
                message: cause.to_string(),
                cause: Some(cause),
            }),
        )
        .into();
        assert_eq!(result.consensus_code, 41103);
        assert_eq!(
            result.consensus_kind,
            PlatformWalletFFIConsensusErrorKind::State
        );
    }
}
