//! FFI binding for `IdentityWallet::token_update_config_with_external_signer`.
//!
//! `TokenConfigurationChangeItem` has 32 variants. Surfacing each one
//! through its own FFI parameter list would be both noisy and a
//! moving target — every new variant added to the enum would force a
//! new entry point. Instead, the FFI takes a `(tag, payload_json)`
//! pair and dispatches Rust-side. Wave 7 only implements one tag
//! (`0 = MaxSupply`); other tags are rejected with
//! `ErrorInvalidParameter` and a clear message so the caller sees
//! that the entry point exists but the variant is not yet wired.
//!
//! Tag 0 — MaxSupply payload shape:
//! ```json
//! { "newMaxSupply": "1000000" }   // string-encoded u64; null = remove cap
//! ```
//! The amount is a JSON string rather than a number because Platform's
//! `TokenAmount` is a u64 and JSON integers above 2^53 can't be
//! represented exactly by JS-style parsers.

use std::ffi::CStr;
use std::os::raw::c_char;

use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use rs_sdk_ffi::{SignerHandle, VTableSigner};
use serde_json::Value;

use super::group_info::{decode_group_info, GroupInfoDecode};
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;

/// Tag values accepted by `change_item_tag`. Mirrors the
/// `TokenConfigurationChangeItem` variant indices but the FFI keeps
/// its own table so the wire format doesn't drift if `u8_item_index`
/// is ever renumbered.
const TAG_MAX_SUPPLY: u8 = 0;

/// Decode the `(tag, payload_json)` pair into a
/// `TokenConfigurationChangeItem`. Returns the variant or stamps
/// `out_error` and returns the error code to bubble back to the
/// caller. Future tags are added here without changing the surface.
unsafe fn decode_change_item(
    tag: u8,
    payload_json: *const c_char,
    out_error: *mut PlatformWalletFFIError,
) -> Result<TokenConfigurationChangeItem, PlatformWalletFFIResult> {
    match tag {
        TAG_MAX_SUPPLY => decode_max_supply_payload(payload_json, out_error),
        other => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!(
                        "change_item_tag {} not yet supported by FFI (only MaxSupply = 0 is wired in this release)",
                        other
                    ),
                );
            }
            Err(PlatformWalletFFIResult::ErrorInvalidParameter)
        }
    }
}

unsafe fn decode_max_supply_payload(
    payload_json: *const c_char,
    out_error: *mut PlatformWalletFFIError,
) -> Result<TokenConfigurationChangeItem, PlatformWalletFFIResult> {
    if payload_json.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "change_item_payload_json is null (expected JSON object for MaxSupply)",
            );
        }
        return Err(PlatformWalletFFIResult::ErrorNullPointer);
    }
    let payload_str = match CStr::from_ptr(payload_json).to_str() {
        Ok(s) => s,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    format!("change_item_payload_json is not valid UTF-8: {e}"),
                );
            }
            return Err(PlatformWalletFFIResult::ErrorUtf8Conversion);
        }
    };
    let parsed: Value = match serde_json::from_str(payload_str) {
        Ok(v) => v,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorDeserialization,
                    format!("change_item_payload_json is not valid JSON: {e}"),
                );
            }
            return Err(PlatformWalletFFIResult::ErrorDeserialization);
        }
    };

    let new_max_supply_field = match parsed.get("newMaxSupply") {
        Some(v) => v,
        None => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    "MaxSupply payload missing required field 'newMaxSupply'",
                );
            }
            return Err(PlatformWalletFFIResult::ErrorInvalidParameter);
        }
    };

    let new_max_supply: Option<u64> = if new_max_supply_field.is_null() {
        None
    } else if let Some(s) = new_max_supply_field.as_str() {
        match s.parse::<u64>() {
            Ok(v) => Some(v),
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        format!("'newMaxSupply' is not a valid u64: {e}"),
                    );
                }
                return Err(PlatformWalletFFIResult::ErrorInvalidParameter);
            }
        }
    } else {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorInvalidParameter,
                "'newMaxSupply' must be a string-encoded u64 or null",
            );
        }
        return Err(PlatformWalletFFIResult::ErrorInvalidParameter);
    };

    Ok(TokenConfigurationChangeItem::MaxSupply(new_max_supply))
}

/// Update the configuration of the token at `token_position` on
/// `token_contract_id`.
///
/// The change is described by the `(change_item_tag,
/// change_item_payload_json)` pair so the entry point can grow new
/// variants without changing its parameter list. See the module
/// docstring for the per-tag payload shapes.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id`, `token_contract_id` must each point at exactly 32
///   readable bytes.
/// - `change_item_payload_json` must be a NUL-terminated UTF-8 C string
///   when the tag's payload shape requires one (every currently
///   supported tag does).
/// - `public_note` may be NULL; when non-NULL it must be a
///   NUL-terminated UTF-8 C string.
/// - `group_info_action_id` must point at 32 bytes when
///   `group_info_kind == 2`; ignored otherwise (may be NULL).
/// - `signer_handle` must be a valid, non-destroyed handle from
///   `dash_sdk_signer_create_with_ctx`. Caller retains ownership.
/// - `out_error` may be NULL.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_update_config(
    wallet_handle: Handle,
    identity_id: *const u8,
    token_contract_id: *const u8,
    token_position: u16,
    change_item_tag: u8,
    change_item_payload_json: *const c_char,
    public_note: *const c_char,
    group_info_kind: u8,
    group_info_position: u16,
    group_info_action_id: *const u8,
    group_info_action_is_proposer: bool,
    _signing_key_id: u32,
    signer_handle: *mut SignerHandle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if signer_handle.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "signer_handle is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let id = match read_identifier(identity_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid identity_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };
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

    let change_item = match decode_change_item(change_item_tag, change_item_payload_json, out_error)
    {
        Ok(item) => item,
        Err(code) => return code,
    };

    let public_note_str = if public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(public_note).to_str() {
            Ok(s) if s.is_empty() => None,
            Ok(s) => Some(s.to_owned()),
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        format!("public_note is not valid UTF-8: {e}"),
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        }
    };

    let group_info = match decode_group_info(
        group_info_kind,
        group_info_position,
        group_info_action_id,
        group_info_action_is_proposer,
        out_error,
    ) {
        GroupInfoDecode::Ok(value) => value,
        GroupInfoDecode::Err(code) => return code,
    };

    let signer_addr = signer_handle as usize;

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity().clone();
            let result = block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                identity_wallet
                    .token_update_config_with_external_signer(
                        id,
                        contract_id,
                        token_position,
                        change_item,
                        public_note_str,
                        group_info,
                        signer,
                    )
                    .await
            });
            match result {
                Ok(_) => PlatformWalletFFIResult::Success,
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("token_update_config failed: {e}"),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cstr(s: &str) -> std::ffi::CString {
        std::ffi::CString::new(s).unwrap()
    }

    #[test]
    fn decode_max_supply_some() {
        unsafe {
            let payload = cstr(r#"{"newMaxSupply":"1000000"}"#);
            let item =
                decode_change_item(TAG_MAX_SUPPLY, payload.as_ptr(), std::ptr::null_mut()).unwrap();
            match item {
                TokenConfigurationChangeItem::MaxSupply(Some(v)) => assert_eq!(v, 1_000_000),
                other => panic!("expected MaxSupply(Some), got {:?}", other),
            }
        }
    }

    #[test]
    fn decode_max_supply_none_removes_cap() {
        unsafe {
            let payload = cstr(r#"{"newMaxSupply":null}"#);
            let item =
                decode_change_item(TAG_MAX_SUPPLY, payload.as_ptr(), std::ptr::null_mut()).unwrap();
            match item {
                TokenConfigurationChangeItem::MaxSupply(None) => {}
                other => panic!("expected MaxSupply(None), got {:?}", other),
            }
        }
    }

    #[test]
    fn decode_max_supply_missing_field() {
        unsafe {
            let payload = cstr(r#"{}"#);
            let mut err = PlatformWalletFFIError::success();
            let result = decode_change_item(TAG_MAX_SUPPLY, payload.as_ptr(), &mut err);
            match result {
                Err(PlatformWalletFFIResult::ErrorInvalidParameter) => {}
                other => panic!("expected ErrorInvalidParameter, got {:?}", other),
            }
        }
    }

    #[test]
    fn decode_max_supply_non_string() {
        unsafe {
            let payload = cstr(r#"{"newMaxSupply":123}"#);
            let mut err = PlatformWalletFFIError::success();
            let result = decode_change_item(TAG_MAX_SUPPLY, payload.as_ptr(), &mut err);
            match result {
                Err(PlatformWalletFFIResult::ErrorInvalidParameter) => {}
                other => panic!("expected ErrorInvalidParameter, got {:?}", other),
            }
        }
    }

    #[test]
    fn decode_unsupported_tag_rejected() {
        unsafe {
            let payload = cstr(r#"{}"#);
            let mut err = PlatformWalletFFIError::success();
            let result = decode_change_item(1, payload.as_ptr(), &mut err);
            match result {
                Err(PlatformWalletFFIResult::ErrorInvalidParameter) => {}
                other => panic!(
                    "expected ErrorInvalidParameter for unsupported tag, got {:?}",
                    other
                ),
            }
        }
    }

    #[test]
    fn decode_invalid_json() {
        unsafe {
            let payload = cstr("not json");
            let mut err = PlatformWalletFFIError::success();
            let result = decode_change_item(TAG_MAX_SUPPLY, payload.as_ptr(), &mut err);
            match result {
                Err(PlatformWalletFFIResult::ErrorDeserialization) => {}
                other => panic!("expected ErrorDeserialization, got {:?}", other),
            }
        }
    }
}
