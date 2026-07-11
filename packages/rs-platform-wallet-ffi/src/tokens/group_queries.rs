//! FFI bindings for group-action discovery on token contracts.
//!
//! Two read-only entry points: list pending (or closed) proposals on a
//! group, and list which identities have already signed a specific
//! proposal. Both return a UTF-8 JSON document allocated on the Rust
//! side; callers must free it with [`platform_wallet_free_string`]
//! (already exported from `xpub_render.rs`).
//!
//! The JSON shape is intentionally flat and stable so the Swift side
//! can decode it with one `Codable` family. Per-action `params` are
//! keyed off a `"type"` discriminator that mirrors the
//! `TokenEvent::associated_document_type_name()` token-history
//! document names — proposals for variants the iOS co-sign UI doesn't
//! replay yet (`claim`, `transfer`, `configUpdate`, …) come back with
//! `"type": "<name>"` and an empty `"params": {}` so the row still
//! renders.

use std::os::raw::c_char;

use dpp::group::group_action_status::GroupActionStatus;
use dpp::tokens::emergency_action::TokenEmergencyAction;
use platform_wallet::wallet::tokens::{
    group_action_signers_external, pending_group_actions_external, GroupActionEntry,
    GroupActionParams, GroupActionSignerEntry,
};
use serde_json::{json, Value};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Decode a raw `u8` status from the FFI caller into the rs-dpp enum.
fn decode_status(status: u8) -> Result<GroupActionStatus, PlatformWalletFFIResult> {
    match status {
        0 => Ok(GroupActionStatus::ActionActive),
        1 => Ok(GroupActionStatus::ActionClosed),
        other => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("Invalid group action status: {other} (expected 0 or 1)"),
        )),
    }
}

/// Render a group-action `params` payload as a JSON object.
fn render_params(params: &GroupActionParams) -> (String, Value) {
    fn id(b: &dpp::prelude::Identifier) -> String {
        bs58::encode(b.as_bytes()).into_string()
    }
    match params {
        GroupActionParams::Mint {
            amount,
            recipient,
            public_note,
        } => (
            "mint".to_string(),
            json!({
                "amount": amount.to_string(),
                "recipient": id(recipient),
                "publicNote": public_note,
            }),
        ),
        GroupActionParams::Burn {
            amount,
            burn_from,
            public_note,
        } => (
            "burn".to_string(),
            json!({
                "amount": amount.to_string(),
                "burnFrom": id(burn_from),
                "publicNote": public_note,
            }),
        ),
        GroupActionParams::Freeze {
            target,
            public_note,
        } => (
            "freeze".to_string(),
            json!({
                "target": id(target),
                "publicNote": public_note,
            }),
        ),
        GroupActionParams::Unfreeze {
            target,
            public_note,
        } => (
            "unfreeze".to_string(),
            json!({
                "target": id(target),
                "publicNote": public_note,
            }),
        ),
        GroupActionParams::DestroyFrozenFunds {
            target,
            amount,
            public_note,
        } => (
            "destroyFrozenFunds".to_string(),
            json!({
                "target": id(target),
                "amount": amount.to_string(),
                "publicNote": public_note,
            }),
        ),
        GroupActionParams::EmergencyAction {
            action,
            public_note,
        } => {
            let kind = match action {
                TokenEmergencyAction::Pause => "pause",
                TokenEmergencyAction::Resume => "resume",
            };
            (
                kind.to_string(),
                json!({
                    "publicNote": public_note,
                }),
            )
        }
        GroupActionParams::SetPrice {
            price_per_token,
            public_note,
        } => (
            "setPrice".to_string(),
            json!({
                "pricePerToken": price_per_token.map(|p| p.to_string()),
                "publicNote": public_note,
            }),
        ),
        GroupActionParams::DirectPurchase { amount, total_cost } => (
            "directPurchase".to_string(),
            json!({
                "amount": amount.to_string(),
                "totalCost": total_cost.to_string(),
            }),
        ),
        GroupActionParams::Other { name } => (name.clone(), json!({})),
    }
}

fn render_action_entry(entry: &GroupActionEntry) -> Value {
    let (kind, params) = render_params(&entry.params);
    let closed = matches!(entry.status, GroupActionStatus::ActionClosed);
    json!({
        "actionId": bs58::encode(entry.action_id.as_bytes()).into_string(),
        "type": kind,
        "proposer": bs58::encode(entry.proposer.as_bytes()).into_string(),
        "tokenContractPosition": entry.token_contract_position,
        "closed": closed,
        "params": params,
    })
}

fn render_signer_entry(entry: &GroupActionSignerEntry) -> Value {
    json!({
        "identityId": bs58::encode(entry.identity_id.as_bytes()).into_string(),
        "power": entry.power,
    })
}

/// Allocate a JSON `Value` as a NUL-terminated C string and write it
/// to `out_json`. Caller frees with `platform_wallet_free_string`.
unsafe fn emit_json(value: Value, out_json: *mut *mut c_char) -> PlatformWalletFFIResult {
    let serialized = unwrap_result_or_return!(serde_json::to_string(&value));
    let cstring = unwrap_result_or_return!(std::ffi::CString::new(serialized));
    *out_json = cstring.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Fetch group-action proposals on `(token_contract_id, group_contract_position)`
/// filtered by `status` (0 = pending / active, 1 = closed). Writes a
/// JSON array to `out_json`.
///
/// JSON element shape:
/// ```json
/// {
///   "actionId": "<base58 32-byte id>",
///   "type": "mint" | "burn" | "freeze" | "unfreeze"
///         | "destroyFrozenFunds" | "pause" | "resume"
///         | "setPrice" | "directPurchase" | "<other>",
///   "proposer": "<base58 identity id>",
///   "tokenContractPosition": <u16>,
///   "closed": <bool>,
///   "params": { ...per-variant payload... }
/// }
/// ```
///
/// `params` carries the per-variant payload, e.g. for mint:
/// `{ "amount": "...", "recipient": "...", "publicNote": "..." }`.
///
/// `amount` / `pricePerToken` / `totalCost` are emitted as JSON
/// strings to dodge JS's 53-bit integer precision cliff. Swift
/// decodes them straight into `UInt64` via a helper.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `token_contract_id` must point at exactly 32 readable bytes.
/// - `start_at_action_id` may be NULL; when non-NULL it must point at
///   exactly 32 readable bytes.
/// - `out_json` must be a writable `*mut *mut c_char`. On success the
///   caller owns the string and must free it via
///   `platform_wallet_free_string`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_pending_group_actions(
    wallet_handle: Handle,
    token_contract_id: *const u8,
    group_contract_position: u16,
    status: u8,
    start_at_action_id: *const u8,
    limit: u16,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_json);
    *out_json = std::ptr::null_mut();

    let contract_id = unwrap_result_or_return!(read_identifier(token_contract_id));

    let status_enum = unwrap_result_or_return!(decode_status(status));

    let start_at = if start_at_action_id.is_null() {
        None
    } else {
        Some((
            unwrap_result_or_return!(read_identifier(start_at_action_id)),
            true,
        ))
    };

    let limit_opt = if limit == 0 { None } else { Some(limit) };

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let sdk = wallet.sdk_arc();
        block_on_worker(async move {
            pending_group_actions_external(
                sdk.as_ref(),
                contract_id,
                group_contract_position,
                status_enum,
                start_at,
                limit_opt,
            )
            .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let entries = unwrap_result_or_return!(result);
    let array: Vec<Value> = entries.iter().map(render_action_entry).collect();
    emit_json(Value::Array(array), out_json)
}

/// Fetch the signers of a specific group-action proposal. Writes a
/// JSON array to `out_json`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_group_action_signers(
    wallet_handle: Handle,
    token_contract_id: *const u8,
    group_contract_position: u16,
    status: u8,
    action_id: *const u8,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_json);
    *out_json = std::ptr::null_mut();

    let contract_id = unwrap_result_or_return!(read_identifier(token_contract_id));
    let action_id_decoded = unwrap_result_or_return!(read_identifier(action_id));

    let status_enum = unwrap_result_or_return!(decode_status(status));

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let sdk = wallet.sdk_arc();
        block_on_worker(async move {
            group_action_signers_external(
                sdk.as_ref(),
                contract_id,
                group_contract_position,
                status_enum,
                action_id_decoded,
            )
            .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let entries = unwrap_result_or_return!(result);
    let array: Vec<Value> = entries.iter().map(render_signer_entry).collect();
    emit_json(Value::Array(array), out_json)
}
