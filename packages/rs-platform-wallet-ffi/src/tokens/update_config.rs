//! FFI binding for `IdentityWallet::token_update_config_with_external_signer`.

use std::ffi::CStr;
use std::os::raw::c_char;

use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use rs_sdk_ffi::{SignerHandle, VTableSigner};
use serde_json::Value;

use super::group_info::decode_group_info;
use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

const TAG_MAX_SUPPLY: u8 = 0;

unsafe fn decode_change_item(
    tag: u8,
    payload_json: *const c_char,
) -> Result<TokenConfigurationChangeItem, PlatformWalletFFIResult> {
    match tag {
        TAG_MAX_SUPPLY => decode_max_supply_payload(payload_json),
        other => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!(
                "change_item_tag {other} not yet supported by FFI (only MaxSupply = 0 is wired in this release)"
            ),
        )),
    }
}

unsafe fn decode_max_supply_payload(
    payload_json: *const c_char,
) -> Result<TokenConfigurationChangeItem, PlatformWalletFFIResult> {
    if payload_json.is_null() {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorNullPointer,
            "change_item_payload_json is null (expected JSON object for MaxSupply)",
        ));
    }
    let payload_str = CStr::from_ptr(payload_json).to_str()?;
    let parsed: Value = serde_json::from_str(payload_str)?;

    let new_max_supply_field = match parsed.get("newMaxSupply") {
        Some(v) => v,
        None => {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "MaxSupply payload missing required field 'newMaxSupply'",
            ));
        }
    };

    let new_max_supply: Option<u64> = match new_max_supply_field {
        Value::Null => None,
        Value::String(s) => Some(s.parse::<u64>().map_err(|e| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("'newMaxSupply' is not a valid u64: {e}"),
            )
        })?),
        _ => {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "'newMaxSupply' must be a string-encoded u64 or null",
            ));
        }
    };

    Ok(TokenConfigurationChangeItem::MaxSupply(new_max_supply))
}

/// Update the configuration of the token at `token_position` on `token_contract_id`.
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
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);

    let id = unwrap_result_or_return!(read_identifier(identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(token_contract_id));

    let change_item = unwrap_result_or_return!(decode_change_item(
        change_item_tag,
        change_item_payload_json
    ));

    let public_note_str = if public_note.is_null() {
        None
    } else {
        {
            let s = unwrap_result_or_return!(CStr::from_ptr(public_note).to_str());
            if s.is_empty() {
                None
            } else {
                Some(s.to_owned())
            }
        }
    };

    let group_info = unwrap_result_or_return!(decode_group_info(
        group_info_kind,
        group_info_position,
        group_info_action_id,
        group_info_action_is_proposer,
    ));

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
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
        })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
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
            let item = decode_change_item(TAG_MAX_SUPPLY, payload.as_ptr()).unwrap();
            match item {
                TokenConfigurationChangeItem::MaxSupply(Some(v)) => assert_eq!(v, 1_000_000),
                other => panic!("expected MaxSupply(Some), got {other:?}"),
            }
        }
    }

    #[test]
    fn decode_max_supply_none_removes_cap() {
        unsafe {
            let payload = cstr(r#"{"newMaxSupply":null}"#);
            let item = decode_change_item(TAG_MAX_SUPPLY, payload.as_ptr()).unwrap();
            match item {
                TokenConfigurationChangeItem::MaxSupply(None) => {}
                other => panic!("expected MaxSupply(None), got {other:?}"),
            }
        }
    }

    #[test]
    fn decode_max_supply_missing_field() {
        unsafe {
            let payload = cstr(r#"{}"#);
            let result = decode_change_item(TAG_MAX_SUPPLY, payload.as_ptr());
            match result {
                Err(r) if r.code == PlatformWalletFFIResultCode::ErrorInvalidParameter => {}
                _ => panic!("expected ErrorInvalidParameter"),
            }
        }
    }

    #[test]
    fn decode_unsupported_tag_rejected() {
        unsafe {
            let payload = cstr(r#"{}"#);
            let result = decode_change_item(1, payload.as_ptr());
            match result {
                Err(r) if r.code == PlatformWalletFFIResultCode::ErrorInvalidParameter => {}
                _ => panic!("expected ErrorInvalidParameter for unsupported tag"),
            }
        }
    }

    #[test]
    fn decode_invalid_json() {
        unsafe {
            let payload = cstr("not json");
            let result = decode_change_item(TAG_MAX_SUPPLY, payload.as_ptr());
            match result {
                Err(r) if r.code == PlatformWalletFFIResultCode::ErrorDeserialization => {}
                _ => panic!("expected ErrorDeserialization"),
            }
        }
    }
}
