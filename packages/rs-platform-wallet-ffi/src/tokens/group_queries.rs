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

use crate::error::{PlatformWalletFFIError, PlatformWalletFFIResult};
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;

/// Decode a raw `u8` status from the FFI caller into the rs-dpp enum.
/// Mirrors `GroupActionStatus::try_from(u8)` but stamps `out_error`
/// on failure rather than returning `anyhow::Error`.
unsafe fn decode_status(
    status: u8,
    out_error: *mut PlatformWalletFFIError,
) -> Result<GroupActionStatus, PlatformWalletFFIResult> {
    match status {
        0 => Ok(GroupActionStatus::ActionActive),
        1 => Ok(GroupActionStatus::ActionClosed),
        other => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("Invalid group action status: {other} (expected 0 or 1)"),
                );
            }
            Err(PlatformWalletFFIResult::ErrorInvalidParameter)
        }
    }
}

/// Render a group-action `params` payload as a JSON object. Each
/// variant matches the discriminator names emitted in
/// [`render_action_entry`] below. `kind` is owned so the `Other`
/// variant can carry a runtime name without a static-lifetime trick.
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
unsafe fn emit_json(
    value: Value,
    out_json: *mut *mut c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let serialized = match serde_json::to_string(&value) {
        Ok(s) => s,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorSerialization,
                    format!("Failed to serialize group action JSON: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorSerialization;
        }
    };
    let cstring = match std::ffi::CString::new(serialized) {
        Ok(s) => s,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorSerialization,
                    format!("Group action JSON contained a NUL byte: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorSerialization;
        }
    };
    *out_json = cstring.into_raw();
    PlatformWalletFFIResult::Success
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
/// - `out_error` may be NULL.
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
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_json.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "out_json is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    *out_json = std::ptr::null_mut();

    let contract_id = match read_identifier(token_contract_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid token_contract_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    let status_enum = match decode_status(status, out_error) {
        Ok(s) => s,
        Err(code) => return code,
    };

    let start_at = if start_at_action_id.is_null() {
        None
    } else {
        match read_identifier(start_at_action_id) {
            Ok(i) => Some((i, true)),
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid start_at_action_id: {e}"),
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidIdentifier;
            }
        }
    };

    let limit_opt = if limit == 0 { None } else { Some(limit) };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let sdk = wallet.sdk_arc();
            let result = block_on_worker(async move {
                pending_group_actions_external(
                    sdk.as_ref(),
                    contract_id,
                    group_contract_position,
                    status_enum,
                    start_at,
                    limit_opt,
                )
                .await
            });
            match result {
                Ok(entries) => {
                    let array: Vec<Value> = entries.iter().map(render_action_entry).collect();
                    emit_json(Value::Array(array), out_json, out_error)
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("token_pending_group_actions failed: {e}"),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidHandle,
                    "Invalid platform-wallet handle",
                );
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Fetch the signers of a specific group-action proposal. Writes a
/// JSON array to `out_json`.
///
/// JSON element shape:
/// ```json
/// { "identityId": "<base58 identity id>", "power": <u32> }
/// ```
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `token_contract_id` and `action_id` must each point at exactly
///   32 readable bytes.
/// - `out_json` must be a writable `*mut *mut c_char`. On success the
///   caller owns the string and must free it via
///   `platform_wallet_free_string`.
/// - `out_error` may be NULL.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_group_action_signers(
    wallet_handle: Handle,
    token_contract_id: *const u8,
    group_contract_position: u16,
    status: u8,
    action_id: *const u8,
    out_json: *mut *mut c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_json.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "out_json is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    *out_json = std::ptr::null_mut();

    let contract_id = match read_identifier(token_contract_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid token_contract_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };
    let action_id_decoded = match read_identifier(action_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid action_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    let status_enum = match decode_status(status, out_error) {
        Ok(s) => s,
        Err(code) => return code,
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let sdk = wallet.sdk_arc();
            let result = block_on_worker(async move {
                group_action_signers_external(
                    sdk.as_ref(),
                    contract_id,
                    group_contract_position,
                    status_enum,
                    action_id_decoded,
                )
                .await
            });
            match result {
                Ok(entries) => {
                    let array: Vec<Value> = entries.iter().map(render_signer_entry).collect();
                    emit_json(Value::Array(array), out_json, out_error)
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("token_group_action_signers failed: {e}"),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidHandle,
                    "Invalid platform-wallet handle",
                );
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}
