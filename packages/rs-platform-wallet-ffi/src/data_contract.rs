//! FFI bindings for data-contract create / update operations on
//! `IdentityWallet`, and for previewing what an update changes.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::prelude::Identifier;
use platform_wallet::wallet::identity::network::{
    ContractSectionChanges, DataContractUpdatePreview,
};
use rs_sdk_ffi::{SignerHandle, VTableSigner};
use serde_json::json;

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Create + broadcast a new data contract owned by `owner_identity_id`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_create_data_contract_with_signer(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    documents_schema_json: *const c_char,
    tokens_schema_json: *const c_char,
    groups_schema_json: *const c_char,
    keywords_json: *const c_char,
    description: *const c_char,
    config_json: *const c_char,
    signer_handle: *mut SignerHandle,
    out_contract_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(documents_schema_json);
    check_ptr!(out_contract_id);

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));

    let documents_str = unwrap_result_or_return!(CStr::from_ptr(documents_schema_json).to_str());

    let tokens_str = unwrap_result_or_return!(read_optional_str(tokens_schema_json));
    let groups_str = unwrap_result_or_return!(read_optional_str(groups_schema_json));
    let keywords_str = unwrap_result_or_return!(read_optional_str(keywords_json));
    let description_str = unwrap_result_or_return!(read_optional_str(description));
    let config_str = unwrap_result_or_return!(read_optional_str(config_json));

    let signer_addr = signer_handle as usize;
    let owner_id_for_async = owner_id;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let result: Result<Identifier, _> = block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .create_data_contract_with_signer(
                    &owner_id_for_async,
                    documents_str,
                    tokens_str,
                    groups_str,
                    keywords_str,
                    description_str,
                    config_str,
                    signer,
                )
                .await
                .map(|contract| contract.id())
        });
        result
    });
    let result = unwrap_option_or_return!(option);
    let contract_id = unwrap_result_or_return!(result);
    let bytes = contract_id.to_buffer();
    let dst = slice::from_raw_parts_mut(out_contract_id, 32);
    dst.copy_from_slice(&bytes);
    PlatformWalletFFIResult::ok()
}

/// Update + broadcast an existing data contract owned by
/// `owner_identity_id`. The wallet fetches the live contract, validates
/// the owner, and *merges* the supplied sections onto it at the next
/// contract version (it reads the current version from Platform and
/// bumps it). The JSON args are additive overlays: `documents_schema`
/// / `tokens_schema` / `groups_schema` entries are added or replaced
/// key-by-key (omitted keys keep their on-chain definition), and
/// `keywords` / `description` / `config` override only when supplied —
/// so a single-section update never wipes the rest of the contract.
///
/// `expected_current_version` is the contract version the caller reviewed
/// (the `currentVersion` of `platform_wallet_preview_data_contract_update`).
/// When the contract Platform now holds is at another version the update is
/// refused before any nonce is read, and the caller should review again.
/// `0` means no check: contract versions start at 1
/// (`INITIAL_DATA_CONTRACT_VERSION` in rs-dpp), so `0` is never a real
/// version.
///
/// A broadcast that may have reached Platform but whose result could not be
/// confirmed returns `ErrorTransactionBroadcastUnconfirmed`: the update may
/// have executed.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_update_data_contract_with_signer(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    documents_schema_json: *const c_char,
    tokens_schema_json: *const c_char,
    groups_schema_json: *const c_char,
    keywords_json: *const c_char,
    description: *const c_char,
    config_json: *const c_char,
    expected_current_version: u32,
    signer_handle: *mut SignerHandle,
    out_contract_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(documents_schema_json);
    check_ptr!(out_contract_id);

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));

    let documents_str = unwrap_result_or_return!(CStr::from_ptr(documents_schema_json).to_str());

    let tokens_str = unwrap_result_or_return!(read_optional_str(tokens_schema_json));
    let groups_str = unwrap_result_or_return!(read_optional_str(groups_schema_json));
    let keywords_str = unwrap_result_or_return!(read_optional_str(keywords_json));
    let description_str = unwrap_result_or_return!(read_optional_str(description));
    let config_str = unwrap_result_or_return!(read_optional_str(config_json));
    let expected_version = (expected_current_version != 0).then_some(expected_current_version);

    let signer_addr = signer_handle as usize;
    let owner_id_for_async = owner_id;
    let contract_id_for_async = contract_id_value;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let result: Result<Identifier, _> = block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .update_data_contract_with_signer(
                    &owner_id_for_async,
                    &contract_id_for_async,
                    documents_str,
                    tokens_str,
                    groups_str,
                    keywords_str,
                    description_str,
                    config_str,
                    expected_version,
                    signer,
                )
                .await
                .map(|contract| contract.id())
        });
        result
    });
    let result = unwrap_option_or_return!(option);
    let contract_id = unwrap_result_or_return!(result);
    let bytes = contract_id.to_buffer();
    let dst = slice::from_raw_parts_mut(out_contract_id, 32);
    dst.copy_from_slice(&bytes);
    PlatformWalletFFIResult::ok()
}

/// Report what `platform_wallet_update_data_contract_with_signer`, called
/// with the same arguments, would change in the contract `contract_id`,
/// without signing or broadcasting anything.
///
/// Goes through `IdentityWallet::preview_data_contract_update`: the wallet
/// fetches the live contract, checks that `owner_identity_id` owns it,
/// merges the supplied sections onto it and builds the updated contract as
/// the update does, then compares the two. It consumes no nonce. A section
/// the update would refuse to build (JSON that does not parse, a schema the
/// contract parser rejects) is the same error here. The preview says what
/// changes, not whether Platform accepts the update: the network checks the
/// contract update rules when the update is broadcast.
///
/// On success `*out_preview_json` receives a JSON object; release it with
/// `platform_wallet_string_free`. On any error it is left null. The shape:
///
/// ```json
/// {"currentVersion":1,"newVersion":2,"hasChanges":true,
///  "documentTypes":{"added":["note"],"changed":[]},
///  "tokens":{"added":["0"],"changed":[]},
///  "groups":{"added":[],"changed":[]},
///  "keywordsChanged":false,"descriptionChanged":false,"configChanged":false}
/// ```
///
/// Document types are keyed by name, tokens and groups by contract position
/// (decimal strings). `hasChanges` is false when the update would change
/// nothing besides the version, which every update raises.
///
/// # Safety
/// - `owner_identity_id` and `contract_id` must each point to 32 readable
///   bytes.
/// - `documents_schema_json` must be a NUL-terminated UTF-8 string;
///   `tokens_schema_json`, `groups_schema_json`, `keywords_json`,
///   `description` and `config_json` must each be null or a NUL-terminated
///   UTF-8 string (an empty string counts as null).
/// - `out_preview_json` must be a valid, writable pointer.
///
/// Blocks the calling thread until the fetch finishes, so it must not be
/// called from inside an async runtime. A panic inside is reported as
/// `ErrorWalletOperation` where panics unwind; nothing is signed or
/// broadcast either way.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_preview_data_contract_update(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    documents_schema_json: *const c_char,
    tokens_schema_json: *const c_char,
    groups_schema_json: *const c_char,
    keywords_json: *const c_char,
    description: *const c_char,
    config_json: *const c_char,
    out_preview_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        "platform_wallet_preview_data_contract_update",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        "Nothing was signed or broadcast; review the update again.",
        || unsafe {
            preview_data_contract_update(
                wallet_handle,
                owner_identity_id,
                contract_id,
                documents_schema_json,
                tokens_schema_json,
                groups_schema_json,
                keywords_json,
                description,
                config_json,
                out_preview_json,
            )
        },
    )
}

/// The body of `platform_wallet_preview_data_contract_update`, run inside its
/// panic guard.
///
/// # Safety
/// As `platform_wallet_preview_data_contract_update`.
#[allow(clippy::too_many_arguments)]
unsafe fn preview_data_contract_update(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    documents_schema_json: *const c_char,
    tokens_schema_json: *const c_char,
    groups_schema_json: *const c_char,
    keywords_json: *const c_char,
    description: *const c_char,
    config_json: *const c_char,
    out_preview_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_preview_json);
    *out_preview_json = ptr::null_mut();
    check_ptr!(documents_schema_json);

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));

    let documents_str = unwrap_result_or_return!(CStr::from_ptr(documents_schema_json).to_str());

    let tokens_str = unwrap_result_or_return!(read_optional_str(tokens_schema_json));
    let groups_str = unwrap_result_or_return!(read_optional_str(groups_schema_json));
    let keywords_str = unwrap_result_or_return!(read_optional_str(keywords_json));
    let description_str = unwrap_result_or_return!(read_optional_str(description));
    let config_str = unwrap_result_or_return!(read_optional_str(config_json));

    // Clone the identity wallet under the registry guard and fetch outside
    // it, so a slow network read does not hold the registry lock.
    let option =
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone());
    let identity_wallet = unwrap_option_or_return!(option);
    let result = block_on_worker(async move {
        identity_wallet
            .preview_data_contract_update(
                &owner_id,
                &contract_id_value,
                documents_str,
                tokens_str,
                groups_str,
                keywords_str,
                description_str,
                config_str,
            )
            .await
    });
    let preview = unwrap_result_or_return!(result);

    // A NUL byte inside the JSON would be a serializer bug; refuse it rather
    // than panic across the FFI boundary.
    let json = preview_to_json(&preview).to_string();
    *out_preview_json = unwrap_result_or_return!(CString::new(json)).into_raw();
    PlatformWalletFFIResult::ok()
}

/// `preview` in the JSON shape `platform_wallet_preview_data_contract_update`
/// documents.
fn preview_to_json(preview: &DataContractUpdatePreview) -> serde_json::Value {
    json!({
        "currentVersion": preview.current_version,
        "newVersion": preview.new_version,
        "hasChanges": preview.has_changes(),
        "documentTypes": section_changes_to_json(&preview.document_types),
        "tokens": section_changes_to_json(&preview.tokens),
        "groups": section_changes_to_json(&preview.groups),
        "keywordsChanged": preview.keywords_changed,
        "descriptionChanged": preview.description_changed,
        "configChanged": preview.config_changed,
    })
}

/// One section of a preview as `{"added":[...],"changed":[...]}`.
fn section_changes_to_json(changes: &ContractSectionChanges) -> serde_json::Value {
    json!({
        "added": changes.added,
        "changed": changes.changed,
    })
}

/// Decode an optional NUL-terminated UTF-8 C string.
unsafe fn read_optional_str<'a>(
    ptr: *const c_char,
) -> Result<Option<&'a str>, PlatformWalletFFIResult> {
    if ptr.is_null() {
        return Ok(None);
    }
    let s = CStr::from_ptr(ptr).to_str()?;
    Ok(if s.is_empty() { None } else { Some(s) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(added: &[&str], changed: &[&str]) -> ContractSectionChanges {
        ContractSectionChanges {
            added: added.iter().map(|key| key.to_string()).collect(),
            changed: changed.iter().map(|key| key.to_string()).collect(),
        }
    }

    #[test]
    fn should_write_every_preview_field_in_the_documented_shape() {
        let preview = DataContractUpdatePreview {
            current_version: 1,
            new_version: 2,
            document_types: section(&["comment"], &["note"]),
            tokens: section(&["2", "10"], &[]),
            groups: section(&[], &["0"]),
            keywords_changed: true,
            description_changed: false,
            config_changed: true,
        };

        assert_eq!(
            preview_to_json(&preview),
            json!({
                "currentVersion": 1,
                "newVersion": 2,
                "hasChanges": true,
                "documentTypes": { "added": ["comment"], "changed": ["note"] },
                "tokens": { "added": ["2", "10"], "changed": [] },
                "groups": { "added": [], "changed": ["0"] },
                "keywordsChanged": true,
                "descriptionChanged": false,
                "configChanged": true,
            })
        );
    }

    #[test]
    fn should_take_has_changes_from_the_wallet_preview() {
        let preview = DataContractUpdatePreview {
            current_version: 4,
            new_version: 5,
            document_types: ContractSectionChanges::default(),
            tokens: ContractSectionChanges::default(),
            groups: ContractSectionChanges::default(),
            keywords_changed: false,
            description_changed: false,
            config_changed: false,
        };

        let json = preview_to_json(&preview);

        assert_eq!(json["hasChanges"], json!(preview.has_changes()));
        assert_eq!(json["hasChanges"], json!(false));
        assert_eq!(json["documentTypes"], json!({ "added": [], "changed": [] }));
    }
}
