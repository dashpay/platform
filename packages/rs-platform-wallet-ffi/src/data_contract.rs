//! FFI bindings for data-contract create / update operations on
//! `IdentityWallet`.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::prelude::Identifier;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

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
