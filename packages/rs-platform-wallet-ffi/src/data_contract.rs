//! FFI bindings for data-contract create / update operations on
//! `IdentityWallet`. Thin C-ABI shells over the
//! `IdentityWallet::create_data_contract_with_signer` etc. methods
//! in `platform-wallet`.
//!
//! Replaces the contract-create surface that used to live in
//! `rs-sdk-ffi::data_contract::dash_sdk_data_contract_create` +
//! `dash_sdk_data_contract_put_to_platform_and_wait`. The previous
//! path went through the rs-sdk-ffi tokio runtime, which uses iOS's
//! ~512 KB default thread stack — proof verification in `rs-drive`
//! recurses through GroveDB deeply enough to blow past that and
//! crash with `EXC_BAD_ACCESS` at the function prologue of
//! `grovedb_query::proofs::encoding::Op::decode`. The
//! platform-wallet runtime (`runtime.rs`) configures a 8 MB worker
//! stack precisely for this case.
//!
//! Architectural note: contract creation belongs on platform-wallet
//! (per `swift-sdk/CLAUDE.md`'s "high-level operations go through
//! platform-wallet" rule) because it spans an identity (the owner),
//! needs the wallet's signer, and changes local state the persister
//! tracks.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::prelude::Identifier;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;

/// Create + broadcast a new data contract owned by
/// `owner_identity_id`. Returns the 32-byte contract id in
/// `out_contract_id` on success.
///
/// JSON shapes (all optional except documents):
/// - `documents_schema_json`: object keyed by document type name,
///   each value a JSON Schema. Pass `"{}"` for token-only contracts.
/// - `tokens_schema_json`: object keyed by stringified slot index
///   (`"0"`, `"1"`, …), each value a `TokenConfiguration` JSON.
///   Caller must include `$formatVersion: "0"` tags at the
///   `TokenConfiguration` / `TokenConfigurationConvention` /
///   `TokenConfigurationLocalization` levels — the V1 struct's
///   tagged enums require them.
/// - `groups_schema_json`: object keyed by stringified group
///   position, each value a `Group` JSON.
/// - `keywords_json`: JSON array of strings.
/// - `description`: plain (non-JSON) string.
/// - `config_json`: `DataContractConfig` JSON. The library injects
///   `$formatVersion: "0"` if missing so a Swift-shaped flags dict
///   round-trips cleanly.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle
///   registry.
/// - `owner_identity_id` must point at a 32-byte buffer for the
///   duration of the call.
/// - `documents_schema_json` must be a valid NUL-terminated UTF-8
///   C string.
/// - The other JSON pointers may be NULL; when non-NULL each must
///   be NUL-terminated UTF-8.
/// - `signer_handle` must be a valid, non-destroyed handle from
///   `dash_sdk_signer_create_with_ctx`. Caller retains ownership.
/// - `out_contract_id` must be a valid, writable 32-byte buffer.
/// - `out_error` may be NULL.
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
    if documents_schema_json.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "documents_schema_json is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    if out_contract_id.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "out_contract_id is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // Owner identity id (32 bytes).
    let owner_id = match read_identifier(owner_identity_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid owner_identity_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    // Decode each JSON-string parameter on the calling thread —
    // the resulting `&str` references stay borrowed from the
    // caller's buffers, so we can hand them straight to the
    // library function without allocating.
    let documents_str = match CStr::from_ptr(documents_schema_json).to_str() {
        Ok(s) => s,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    format!("documents_schema_json is not valid UTF-8: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };

    // Optional JSON pointers — `read_optional_str` returns
    // `Ok(None)` for NULL or empty so the library's `Option<&str>`
    // contract is honoured.
    let tokens_str = match read_optional_str(tokens_schema_json, "tokens_schema_json", out_error) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let groups_str = match read_optional_str(groups_schema_json, "groups_schema_json", out_error) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let keywords_str = match read_optional_str(keywords_json, "keywords_json", out_error) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let description_str = match read_optional_str(description, "description", out_error) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let config_str = match read_optional_str(config_json, "config_json", out_error) {
        Ok(s) => s,
        Err(code) => return code,
    };

    // The signer reference outlives the awaited work because the
    // FFI caller pins the `KeychainSigner` (and thus the signer
    // handle) for the duration of this call.
    let signer_addr = signer_handle as usize;
    let owner_id_for_async = owner_id;

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
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
            match result {
                Ok(contract_id) => {
                    let bytes = contract_id.to_buffer();
                    let dst = slice::from_raw_parts_mut(out_contract_id, 32);
                    dst.copy_from_slice(&bytes);
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("create_data_contract_with_signer failed: {e}"),
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

/// Decode an optional NUL-terminated UTF-8 C string. NULL or empty
/// pointer yields `Ok(None)`. Non-empty pointer that fails UTF-8
/// validation yields `Err(error_code)` after stamping `out_error`.
unsafe fn read_optional_str<'a>(
    ptr: *const c_char,
    field_name: &str,
    out_error: *mut PlatformWalletFFIError,
) -> Result<Option<&'a str>, PlatformWalletFFIResult> {
    if ptr.is_null() {
        return Ok(None);
    }
    match CStr::from_ptr(ptr).to_str() {
        Ok(s) => Ok(if s.is_empty() { None } else { Some(s) }),
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    format!("{field_name} is not valid UTF-8: {e}"),
                );
            }
            Err(PlatformWalletFFIResult::ErrorUtf8Conversion)
        }
    }
}
