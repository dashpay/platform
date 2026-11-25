//! FFI binding for `IdentityWallet::token_update_config_with_external_signer`.

use std::ffi::CStr;
use std::os::raw::c_char;

use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::version::{PlatformVersion, TryIntoPlatformVersioned};
use platform_wallet::TokenConfigUpdateOutcome;
use rs_sdk_ffi::{SignerHandle, VTableSigner};
use serde_json::{json, Value};

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

/// The JSON `platform_wallet_token_update_config_and_refresh` writes for
/// `outcome`, at the SDK's protocol version `platform_version`:
///
/// - `{"outcome":"pending"}`: a group action still waiting for signatures.
/// - `{"outcome":"applied","contract":{…},"serializationHex":"…"}`: the
///   contract the update produced, as the contract fetch returns it (JSON in
///   its serialization format, and its platform serialization).
/// - `{"outcome":"appliedNotRefreshed","minimumVersion":N,"reason":"…"}`.
fn outcome_json(
    outcome: TokenConfigUpdateOutcome,
    platform_version: &PlatformVersion,
) -> Result<Value, PlatformWalletFFIResult> {
    Ok(match outcome {
        TokenConfigUpdateOutcome::Pending => json!({ "outcome": "pending" }),
        TokenConfigUpdateOutcome::Applied {
            contract,
            serialization,
        } => {
            let format: DataContractInSerializationFormat = contract
                .try_into_platform_versioned(platform_version)
                .map_err(|e| {
                    PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorSerialization,
                        format!("Failed to convert the contract to its serialization format: {e}"),
                    )
                })?;
            json!({
                "outcome": "applied",
                "contract": serde_json::to_value(&format)?,
                "serializationHex": hex::encode(serialization),
            })
        }
        TokenConfigUpdateOutcome::AppliedNotRefreshed {
            minimum_version,
            reason,
        } => json!({
            "outcome": "appliedNotRefreshed",
            "minimumVersion": minimum_version,
            "reason": reason,
        }),
    })
}

/// Update the configuration of the token at `token_position` on
/// `token_contract_id` (the parameters of `platform_wallet_token_update_config`),
/// then report what the update did.
///
/// # Safety
/// - `out_json` must be a writable `*mut *mut c_char`. On success it receives
///   the outcome JSON described at [`outcome_json`]; free it with
///   `platform_wallet_string_free`. On error it is left null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_update_config_and_refresh(
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
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(out_json);
    *out_json = std::ptr::null_mut();

    let id = unwrap_result_or_return!(read_identifier(identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(token_contract_id));
    let change_item = unwrap_result_or_return!(decode_change_item(
        change_item_tag,
        change_item_payload_json
    ));
    let public_note_str = if public_note.is_null() {
        None
    } else {
        let s = unwrap_result_or_return!(CStr::from_ptr(public_note).to_str());
        (!s.is_empty()).then(|| s.to_owned())
    };
    let group_info = unwrap_result_or_return!(decode_group_info(
        group_info_kind,
        group_info_position,
        group_info_action_id,
        group_info_action_is_proposer,
    ));

    let signer_addr = signer_handle as usize;
    // Clone the identity wallet under the registry guard and run the update
    // and its contract refetch outside it, so the broadcast, the proofs and
    // the retry waits do not hold the registry lock.
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        (wallet.identity().clone(), wallet.sdk().version())
    });
    let (identity_wallet, platform_version) = unwrap_option_or_return!(option);
    let result = block_on_worker(async move {
        let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
        identity_wallet
            .token_update_config_and_refresh_with_external_signer(
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
    let outcome = unwrap_result_or_return!(result);
    let value = unwrap_result_or_return!(outcome_json(outcome, platform_version));
    let serialized = unwrap_result_or_return!(serde_json::to_string(&value));
    let cstring = unwrap_result_or_return!(std::ffi::CString::new(serialized));
    *out_json = cstring.into_raw();
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::tests::fixtures::get_data_contract_fixture;

    #[test]
    fn should_write_pending_and_not_refreshed_outcomes() {
        let version = PlatformVersion::latest();
        assert_eq!(
            outcome_json(TokenConfigUpdateOutcome::Pending, version).unwrap(),
            json!({ "outcome": "pending" })
        );
        assert_eq!(
            outcome_json(
                TokenConfigUpdateOutcome::AppliedNotRefreshed {
                    minimum_version: 3,
                    reason: "behind".to_string(),
                },
                version
            )
            .unwrap(),
            json!({ "outcome": "appliedNotRefreshed", "minimumVersion": 3, "reason": "behind" })
        );
    }

    #[test]
    fn should_write_the_applied_contract_with_its_serialization() {
        let version = PlatformVersion::latest();
        let contract =
            get_data_contract_fixture(None, 0, version.protocol_version).data_contract_owned();
        let value = outcome_json(
            TokenConfigUpdateOutcome::Applied {
                contract,
                serialization: vec![0xab, 0x01],
            },
            version,
        )
        .unwrap();
        assert_eq!(value["outcome"], "applied");
        assert_eq!(value["serializationHex"], "ab01");
        assert!(value["contract"]["id"].is_string(), "{value}");
    }

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
