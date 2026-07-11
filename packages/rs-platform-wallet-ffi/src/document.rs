//! FFI bindings for document create operations on `IdentityWallet`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dpp::document::{Document, DocumentV0Getters};
use dpp::prelude::Identifier;
use dpp::serialization::ValueConvertible;
use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::{PlatformWalletError, TxMetadataKeySource};
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Select the txMetadata key-derivation source for `wallet` by capability —
/// the same two-phase convention as `identity_key_preview` /
/// `identity_discovery`:
///
/// - a wallet with resident private keys (mnemonic / seed / xprv) derives
///   in-process; the resolver is never touched (returns `Ok(None)`);
/// - an external-signable / watch-only wallet (the Android/iOS apps — no
///   in-process private keys, so the resident derive fails with `External
///   signable wallet has no private key`) requires the host mnemonic
///   resolver: the wallet's mnemonic is resolved on demand (keyed by the
///   wallet's own id) and returned as a master xprv (`Ok(Some(master))`).
///   The CALLER must wipe it (`master.private_key.non_secure_erase()`) once
///   the derive is done. When the resolver handle is null for this shape,
///   errors with a hint naming the requirement.
///
/// The wallet-manager read guard is scoped to the capability check only and
/// is NEVER held across the host resolver callback (which synchronously
/// re-enters Kotlin/Swift and can stall on Keychain/Keystore access).
///
/// # Safety
/// `mnemonic_resolver_handle`, when non-null, must come from
/// [`rs_sdk_ffi::dash_sdk_mnemonic_resolver_create`] and remain valid for the
/// duration of the call.
unsafe fn tx_metadata_key_master_for_wallet(
    wallet: &platform_wallet::PlatformWallet,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
) -> Result<Option<ExtendedPrivKey>, PlatformWalletFFIResult> {
    // Phase 1 — short capability-check guard, dropped before any resolver
    // interaction.
    let wallet_has_resident_keys = {
        let wm = wallet.wallet_manager().blocking_read();
        match wm.get_wallet(&wallet.wallet_id()) {
            Some(kw) => !kw.is_external_signable() && !kw.is_watch_only(),
            None => {
                return Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidHandle,
                    "Wallet not found in wallet manager",
                ));
            }
        }
    };
    if wallet_has_resident_keys {
        return Ok(None);
    }
    if mnemonic_resolver_handle.is_null() {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "this wallet has no resident private keys (external-signable / watch-only); \
             a mnemonic resolver handle is required to derive its txMetadata encryption keys",
        ));
    }
    let wallet_id = wallet.wallet_id();
    // SAFETY: handle is non-null (checked) and the caller's safety contract
    // guarantees it came from `dash_sdk_mnemonic_resolver_create`.
    let master = unsafe {
        resolve_master_from_resolver(mnemonic_resolver_handle, &wallet_id, wallet.network())?
    };
    Ok(Some(master))
}

/// Create + broadcast a new document on `contract_id`'s
/// `document_type_name`, owned by `owner_identity_id`, signed via the
/// external `signer_handle`.
///
/// Goes through `IdentityWallet::create_document_with_signer`, which
/// fetches the on-chain contract, builds a revision-1 document from the
/// supplied `properties_json`, selects an AUTHENTICATION + ECDSA key
/// from the in-process `IdentityManager` whose security level satisfies
/// the document type's requirement, broadcasts on the platform-wallet
/// 8 MB worker stack (required to avoid the GroveDB proof-verification
/// stack overflow), and waits for the confirmed document.
///
/// On success the confirmed document's 32-byte id is written to
/// `out_document_id`, and a NUL-terminated, owned UTF-8 JSON string of
/// the confirmed document is written to `*out_document_json`. The JSON
/// is the canonical query-side representation — the same bytes a DOC-01
/// list query (`dash_sdk_document_search`) returns, produced by
/// re-serializing DPP's canonical value form (`to_object()`) through
/// `serde_json`: `$id`/`$ownerId`/`$creatorId` as base58 strings, binary
/// properties as base64, `$formatVersion` and unset system fields present.
/// Swift persists this body verbatim so the local cache matches what a
/// DOC-01 query would return, rather than the user's form input.
/// Ownership of the JSON transfers to the caller, who MUST release it
/// with `platform_wallet_string_free`. On any error `*out_document_json`
/// is left null.
///
/// The signature never crosses into Swift logic — it routes back
/// through the supplied `signer_handle` (typically
/// `KeychainSigner.handle`); the caller retains ownership of the
/// signer.
///
/// `properties_json` is a NUL-terminated UTF-8 JSON object keyed by
/// property name. Byte-array fields are passed as hex (or base64)
/// strings and identifier fields as base58 (or hex) strings; the
/// schema-driven sanitize step on the Rust side converts them to the
/// protocol's native types. Pass `"{}"` for a document type with no
/// required properties.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_create_document_with_signer(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    properties_json: *const c_char,
    signer_handle: *mut SignerHandle,
    out_document_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(document_type_name);
    check_ptr!(properties_json);
    check_ptr!(out_document_id);
    check_ptr!(out_document_json);

    // Initialize the JSON out-param to null up front so every early
    // error return leaves it null without per-branch bookkeeping.
    *out_document_json = ptr::null_mut();

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));

    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();
    let properties_str = unwrap_result_or_return!(CStr::from_ptr(properties_json).to_str());

    let signer_addr = signer_handle as usize;
    let owner_id_for_async = owner_id;
    let contract_id_for_async = contract_id_value;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let result: Result<(Identifier, String), PlatformWalletError> =
            block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                let confirmed: Document = identity_wallet
                    .create_document_with_signer(
                        &owner_id_for_async,
                        &contract_id_for_async,
                        &document_type_str,
                        properties_str,
                        signer,
                    )
                    .await?;
                // Serialize the confirmed document to its canonical query-side
                // JSON — the same representation a DOC-01 list query returns —
                // so the persisted body matches what a later query yields.
                let json_string = confirmed_document_to_json(&confirmed)?;
                Ok::<_, PlatformWalletError>((confirmed.id(), json_string))
            });
        result
    });
    let result = unwrap_option_or_return!(option);
    let (document_id, document_json) = unwrap_result_or_return!(result);

    // Allocate the owned C string for the JSON body. A NUL byte inside
    // the JSON would be a serializer bug, but guard against it rather
    // than panicking across the FFI boundary.
    let json_cstring = unwrap_result_or_return!(CString::new(document_json));

    let bytes = document_id.to_buffer();
    let dst = slice::from_raw_parts_mut(out_document_id, 32);
    dst.copy_from_slice(&bytes);
    // Transfer ownership of the JSON to the caller (freed via
    // `platform_wallet_string_free`). Written last so the id out-param
    // and the JSON are populated together on the success path.
    *out_document_json = json_cstring.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Serialize a confirmed `Document` to its canonical query-side JSON string —
/// byte-for-byte the representation `dash_sdk_document_search` (the DOC-01 list
/// query) returns. It re-serializes the canonical value form (`to_object()`)
/// through `serde_json`, so `$id`/`$ownerId`/`$creatorId` render as base58
/// strings, binary properties as base64, and `$formatVersion` plus unset system
/// fields (as `null`) are present. Swift persists this verbatim so the local
/// cache matches what a subsequent query returns.
///
/// Note: `dash_sdk_document_get_info` (single-document fetch) emits a different,
/// per-field shape (bytes as hex); this parity is with the list-query path only.
fn confirmed_document_to_json(document: &Document) -> Result<String, PlatformWalletError> {
    let document_value = document.to_object().map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to convert confirmed document to a value: {e}"
        ))
    })?;
    let json_value = serde_json::to_value(&document_value).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to convert confirmed document to JSON: {e}"
        ))
    })?;
    serde_json::to_string(&json_value).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to serialize confirmed document JSON: {e}"
        ))
    })
}

/// Create + broadcast an ENCRYPTED wallet-contract document (the wire-
/// compatible `txMetadata` shape) on `contract_id`'s `document_type_name`,
/// owned by `owner_identity_id`, signed via the external `signer_handle`.
///
/// Goes through `IdentityWallet::create_encrypted_document_with_signer`: the
/// SDK selects the identity's ENCRYPTION key id (the `keyIndex` field), derives
/// the AES key from the wallet HD tree, seals the opaque `payload` into the
/// legacy `version ‖ IV ‖ AES-256-CBC` blob, and writes
/// `{keyIndex, encryptionKeyIndex, encryptedMetadata}` — decryptable by the
/// legacy `org.dashj.platform` stack and vice versa.
///
/// The AES key source is selected by the wallet's capability: a key-resident
/// wallet derives in-process; an external-signable / watch-only wallet (the
/// Android/iOS apps) derives through `mnemonic_resolver_handle` — required
/// non-null for that shape, ignored otherwise (see
/// `tx_metadata_key_master_for_wallet`).
///
/// The caller supplies `encryption_key_index` (the app's per-document index —
/// batching stays app-side), `version` (`1` = protobuf, as the wallet writes),
/// and the already-serialized opaque `payload` (a protobuf `TxMetadataBatch`;
/// the SDK does not parse it). `payload` may be null only when
/// `payload_len == 0`.
///
/// On success the confirmed document's 32-byte id is written to
/// `out_document_id` and its canonical query-side JSON to `*out_document_json`
/// (release with `platform_wallet_string_free`; left null on any error).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_create_encrypted_document_with_signer(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    encryption_key_index: u32,
    version: u8,
    payload: *const u8,
    payload_len: usize,
    signer_handle: *mut SignerHandle,
    out_document_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(document_type_name);
    check_ptr!(out_document_id);
    check_ptr!(out_document_json);

    *out_document_json = ptr::null_mut();

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));
    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();

    // Copy the payload into an owned Vec (it is moved into the async block; a
    // borrow of `payload` can't outlive this call). Null is allowed only for a
    // zero-length payload.
    let payload_vec: Vec<u8> = if payload_len == 0 {
        Vec::new()
    } else {
        check_ptr!(payload);
        slice::from_raw_parts(payload, payload_len).to_vec()
    };

    let signer_addr = signer_handle as usize;
    let owner_id_for_async = owner_id;
    let contract_id_for_async = contract_id_value;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();

        // Key-source selection by wallet capability (may synchronously call
        // back into the host mnemonic resolver for external-signable
        // wallets — see `tx_metadata_key_master_for_wallet`).
        let master_opt =
            unsafe { tx_metadata_key_master_for_wallet(wallet, mnemonic_resolver_handle) }?;

        let result: Result<(Identifier, String), PlatformWalletError> =
            block_on_worker(async move {
                let key_source = match master_opt.as_ref() {
                    Some(master) => TxMetadataKeySource::Master(master),
                    None => TxMetadataKeySource::ResidentWallet,
                };
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                let created = identity_wallet
                    .create_encrypted_document_with_signer(
                        &owner_id_for_async,
                        &contract_id_for_async,
                        &document_type_str,
                        encryption_key_index,
                        version,
                        &payload_vec,
                        key_source,
                        signer,
                    )
                    .await;
                // Wipe the resolved master's scalar (external-signable path)
                // before the result crosses back — `ExtendedPrivKey` has no
                // `Drop`/`Zeroize` (same hygiene as `identity_key_preview`).
                if let Some(mut master) = master_opt {
                    master.private_key.non_secure_erase();
                }
                let confirmed: Document = created?;
                let json_string = confirmed_document_to_json(&confirmed)?;
                Ok::<_, PlatformWalletError>((confirmed.id(), json_string))
            });
        result.map_err(PlatformWalletFFIResult::from)
    });
    let result = unwrap_option_or_return!(option);
    let (document_id, document_json) = unwrap_result_or_return!(result);

    let json_cstring = unwrap_result_or_return!(CString::new(document_json));

    let bytes = document_id.to_buffer();
    let dst = slice::from_raw_parts_mut(out_document_id, 32);
    dst.copy_from_slice(&bytes);
    *out_document_json = json_cstring.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Fetch + DECRYPT every encrypted wallet-contract document owned by
/// `owner_identity_id` on `contract_id`'s `document_type_name` updated at or
/// after `since_ms` (epoch-millis).
///
/// Goes through `IdentityWallet::fetch_encrypted_documents` — the wire-
/// compatible read counterpart of the legacy `getTxMetaData(since, key)`. Each
/// document's `encryptedMetadata` blob is decrypted with the identity's derived
/// key; documents that can't be derived/decrypted are skipped (never abort the
/// fetch).
///
/// The AES key source is selected by the wallet's capability: a key-resident
/// wallet derives in-process; an external-signable / watch-only wallet (the
/// Android/iOS apps) derives through `mnemonic_resolver_handle` — required
/// non-null for that shape, ignored otherwise (see
/// `tx_metadata_key_master_for_wallet`).
///
/// On success `*out_documents_json` receives an owned NUL-terminated JSON array
/// (release with `platform_wallet_string_free`; left null on any error). Each
/// element is
/// `{ "id": base58, "ownerId": base58, "keyIndex": u32, "encryptionKeyIndex":
/// u32, "version": u8, "updatedAt": u64|null, "payload": base64 }`, where
/// `payload` is the decrypted, opaque plaintext the caller parses (a protobuf
/// `TxMetadataBatch` for `version == 1`).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_fetch_encrypted_documents(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    since_ms: u64,
    out_documents_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    use base64::Engine;

    check_ptr!(document_type_name);
    check_ptr!(out_documents_json);

    *out_documents_json = ptr::null_mut();

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));
    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();

    let owner_id_for_async = owner_id;
    let contract_id_for_async = contract_id_value;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();

        // Key-source selection by wallet capability (may synchronously call
        // back into the host mnemonic resolver for external-signable
        // wallets — see `tx_metadata_key_master_for_wallet`).
        let master_opt =
            unsafe { tx_metadata_key_master_for_wallet(wallet, mnemonic_resolver_handle) }?;

        let result: Result<Vec<platform_wallet::DecryptedEncryptedDocument>, PlatformWalletError> =
            block_on_worker(async move {
                let key_source = match master_opt.as_ref() {
                    Some(master) => TxMetadataKeySource::Master(master),
                    None => TxMetadataKeySource::ResidentWallet,
                };
                let fetched = identity_wallet
                    .fetch_encrypted_documents(
                        &owner_id_for_async,
                        &contract_id_for_async,
                        &document_type_str,
                        since_ms,
                        key_source,
                    )
                    .await;
                // Wipe the resolved master's scalar (external-signable path)
                // before the result crosses back — `ExtendedPrivKey` has no
                // `Drop`/`Zeroize` (same hygiene as `identity_key_preview`).
                if let Some(mut master) = master_opt {
                    master.private_key.non_secure_erase();
                }
                fetched
            });
        result.map_err(PlatformWalletFFIResult::from)
    });
    let result = unwrap_option_or_return!(option);
    let docs = unwrap_result_or_return!(result);

    let json_array: Vec<serde_json::Value> = docs
        .iter()
        .map(|d| {
            serde_json::json!({
                "id": bs58::encode(d.document_id.to_buffer()).into_string(),
                "ownerId": bs58::encode(d.owner_id.to_buffer()).into_string(),
                "keyIndex": d.key_index,
                "encryptionKeyIndex": d.encryption_key_index,
                "version": d.version,
                "updatedAt": d.updated_at_ms,
                "payload": base64::engine::general_purpose::STANDARD.encode(&d.payload),
            })
        })
        .collect();
    let json_string =
        unwrap_result_or_return!(serde_json::to_string(&serde_json::Value::Array(json_array)));
    let json_cstring = unwrap_result_or_return!(CString::new(json_string));
    *out_documents_json = json_cstring.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Replace + broadcast `document_id`'s properties on `contract_id`'s
/// `document_type_name`, owned by `owner_identity_id`, signed via the
/// external `signer_handle` with key `signing_key_id`.
///
/// Goes through `IdentityWallet::replace_document_with_signer`, which
/// fetches the current document, applies `properties_json` (schema-
/// sanitized), bumps the revision, validates `signing_key_id` is an
/// AUTHENTICATION + ECDSA key on the owner, broadcasts on the
/// platform-wallet 8 MB worker stack, and waits for the confirmed
/// document.
///
/// On success the confirmed document's 32-byte id is written to
/// `out_document_id`, and a NUL-terminated, owned canonical-document
/// JSON string is written to `*out_document_json` (release with
/// `platform_wallet_string_free`). On any error `*out_document_json`
/// is left null. `properties_json` is the full replacement property
/// object, same hex/base58 encoding rules as the create path.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_document_replace(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    properties_json: *const c_char,
    signing_key_id: u32,
    signer_handle: *mut SignerHandle,
    out_document_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(document_type_name);
    check_ptr!(properties_json);
    check_ptr!(out_document_id);
    check_ptr!(out_document_json);
    *out_document_json = ptr::null_mut();

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));
    let document_id_value = unwrap_result_or_return!(read_identifier(document_id));

    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();
    let properties_str = unwrap_result_or_return!(CStr::from_ptr(properties_json).to_str());

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let result: Result<(Identifier, String), PlatformWalletError> =
            block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                let confirmed: Document = identity_wallet
                    .replace_document_with_signer(
                        &owner_id,
                        &contract_id_value,
                        &document_type_str,
                        &document_id_value,
                        properties_str,
                        signing_key_id,
                        signer,
                    )
                    .await?;
                let json_string = confirmed_document_to_json(&confirmed)?;
                Ok::<_, PlatformWalletError>((confirmed.id(), json_string))
            });
        result
    });
    let result = unwrap_option_or_return!(option);
    let (confirmed_id, document_json) = unwrap_result_or_return!(result);

    let json_cstring = unwrap_result_or_return!(CString::new(document_json));
    let bytes = confirmed_id.to_buffer();
    let dst = slice::from_raw_parts_mut(out_document_id, 32);
    dst.copy_from_slice(&bytes);
    *out_document_json = json_cstring.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Delete + broadcast `document_id` on `contract_id`'s
/// `document_type_name`, owned by `owner_identity_id`, signed via the
/// external `signer_handle` with key `signing_key_id`.
///
/// Goes through `IdentityWallet::delete_document_with_signer`. On
/// success the deleted document's 32-byte id is written to
/// `out_document_id`. Delete returns no document body, so there is no
/// JSON out-param — Swift removes the local row by id.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_document_delete(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    signing_key_id: u32,
    signer_handle: *mut SignerHandle,
    out_document_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(document_type_name);
    check_ptr!(out_document_id);

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));
    let document_id_value = unwrap_result_or_return!(read_identifier(document_id));

    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let result: Result<Identifier, PlatformWalletError> = block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let deleted_id: Identifier = identity_wallet
                .delete_document_with_signer(
                    &owner_id,
                    &contract_id_value,
                    &document_type_str,
                    &document_id_value,
                    signing_key_id,
                    signer,
                )
                .await?;
            Ok::<_, PlatformWalletError>(deleted_id)
        });
        result
    });
    let result = unwrap_option_or_return!(option);
    let deleted_id = unwrap_result_or_return!(result);

    let bytes = deleted_id.to_buffer();
    let dst = slice::from_raw_parts_mut(out_document_id, 32);
    dst.copy_from_slice(&bytes);
    PlatformWalletFFIResult::ok()
}

/// Transfer + broadcast `document_id` on `contract_id`'s
/// `document_type_name`, from `owner_identity_id` to `recipient_id`,
/// signed via the external `signer_handle` with key `signing_key_id`.
///
/// Goes through `IdentityWallet::transfer_document_with_signer`. On
/// success the confirmed document's 32-byte id is written to
/// `out_document_id` and its canonical JSON (now reflecting the new
/// owner) to `*out_document_json` (release with
/// `platform_wallet_string_free`). On any error `*out_document_json`
/// is left null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_document_transfer(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    recipient_id: *const u8,
    signing_key_id: u32,
    signer_handle: *mut SignerHandle,
    out_document_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(document_type_name);
    check_ptr!(out_document_id);
    check_ptr!(out_document_json);
    *out_document_json = ptr::null_mut();

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));
    let document_id_value = unwrap_result_or_return!(read_identifier(document_id));
    let recipient_id_value = unwrap_result_or_return!(read_identifier(recipient_id));

    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let result: Result<(Identifier, String), PlatformWalletError> =
            block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                let confirmed: Document = identity_wallet
                    .transfer_document_with_signer(
                        &owner_id,
                        &contract_id_value,
                        &document_type_str,
                        &document_id_value,
                        &recipient_id_value,
                        signing_key_id,
                        signer,
                    )
                    .await?;
                let json_string = confirmed_document_to_json(&confirmed)?;
                Ok::<_, PlatformWalletError>((confirmed.id(), json_string))
            });
        result
    });
    let result = unwrap_option_or_return!(option);
    let (confirmed_id, document_json) = unwrap_result_or_return!(result);

    let json_cstring = unwrap_result_or_return!(CString::new(document_json));
    let bytes = confirmed_id.to_buffer();
    let dst = slice::from_raw_parts_mut(out_document_id, 32);
    dst.copy_from_slice(&bytes);
    *out_document_json = json_cstring.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Set (update) the trade price of `document_id` on `contract_id`'s
/// `document_type_name`, owned by `owner_identity_id`, to `price`
/// credits — signed via the external `signer_handle` with key
/// `signing_key_id`.
///
/// Goes through `IdentityWallet::set_document_price_with_signer`. On
/// success the confirmed document's 32-byte id is written to
/// `out_document_id` and its canonical JSON (now carrying `$price`) to
/// `*out_document_json` (release with `platform_wallet_string_free`).
/// On any error `*out_document_json` is left null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_document_set_price(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    price: u64,
    signing_key_id: u32,
    signer_handle: *mut SignerHandle,
    out_document_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(document_type_name);
    check_ptr!(out_document_id);
    check_ptr!(out_document_json);
    *out_document_json = ptr::null_mut();

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));
    let document_id_value = unwrap_result_or_return!(read_identifier(document_id));

    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let result: Result<(Identifier, String), PlatformWalletError> =
            block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                let confirmed: Document = identity_wallet
                    .set_document_price_with_signer(
                        &owner_id,
                        &contract_id_value,
                        &document_type_str,
                        &document_id_value,
                        price,
                        signing_key_id,
                        signer,
                    )
                    .await?;
                let json_string = confirmed_document_to_json(&confirmed)?;
                Ok::<_, PlatformWalletError>((confirmed.id(), json_string))
            });
        result
    });
    let result = unwrap_option_or_return!(option);
    let (confirmed_id, document_json) = unwrap_result_or_return!(result);

    let json_cstring = unwrap_result_or_return!(CString::new(document_json));
    let bytes = confirmed_id.to_buffer();
    let dst = slice::from_raw_parts_mut(out_document_id, 32);
    dst.copy_from_slice(&bytes);
    *out_document_json = json_cstring.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Purchase + broadcast for-sale `document_id` on `contract_id`'s
/// `document_type_name` for `price` credits, with `purchaser_id` as
/// the buyer (and new owner) — signed via the external `signer_handle`
/// with key `signing_key_id` (resolved on the purchaser).
///
/// Goes through `IdentityWallet::purchase_document_with_signer`. On
/// success the confirmed document's 32-byte id is written to
/// `out_document_id` and its canonical JSON (now owned by the
/// purchaser) to `*out_document_json` (release with
/// `platform_wallet_string_free`). On any error `*out_document_json`
/// is left null. The buyer must differ from the current owner — the
/// caller gates against the self-buy consensus rejection.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_document_purchase(
    wallet_handle: Handle,
    purchaser_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    document_id: *const u8,
    price: u64,
    signing_key_id: u32,
    signer_handle: *mut SignerHandle,
    out_document_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(document_type_name);
    check_ptr!(out_document_id);
    check_ptr!(out_document_json);
    *out_document_json = ptr::null_mut();

    let purchaser_id_value = unwrap_result_or_return!(read_identifier(purchaser_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));
    let document_id_value = unwrap_result_or_return!(read_identifier(document_id));

    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let result: Result<(Identifier, String), PlatformWalletError> =
            block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                let confirmed: Document = identity_wallet
                    .purchase_document_with_signer(
                        &purchaser_id_value,
                        &contract_id_value,
                        &document_type_str,
                        &document_id_value,
                        price,
                        signing_key_id,
                        signer,
                    )
                    .await?;
                let json_string = confirmed_document_to_json(&confirmed)?;
                Ok::<_, PlatformWalletError>((confirmed.id(), json_string))
            });
        result
    });
    let result = unwrap_option_or_return!(option);
    let (confirmed_id, document_json) = unwrap_result_or_return!(result);

    let json_cstring = unwrap_result_or_return!(CString::new(document_json));
    let bytes = confirmed_id.to_buffer();
    let dst = slice::from_raw_parts_mut(out_document_id, 32);
    dst.copy_from_slice(&bytes);
    *out_document_json = json_cstring.into_raw();
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::document::DocumentV0;
    use dpp::platform_value::Value;
    use std::collections::BTreeMap;

    // The confirmed-document JSON handed to Swift must be the same canonical
    // shape the DOC-01 list query (`dash_sdk_document_search`) returns:
    // `$formatVersion` present, identifiers as base58 strings, and binary
    // properties as base64 strings (not u8-arrays). Guards against reverting to
    // the legacy `to_json_with_identifiers_using_bytes` representation.
    #[test]
    fn confirmed_document_json_matches_canonical_query_shape() {
        let mut properties = BTreeMap::new();
        properties.insert(
            "blob".to_string(),
            Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
        );

        let document = Document::V0(DocumentV0 {
            id: Identifier::from([1u8; 32]),
            owner_id: Identifier::from([2u8; 32]),
            properties,
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        let json: serde_json::Value =
            serde_json::from_str(&confirmed_document_to_json(&document).unwrap()).unwrap();

        // `$formatVersion` present (the legacy shape omitted it).
        assert_eq!(json["$formatVersion"], serde_json::json!("0"));
        // Identifiers as base58 strings.
        assert!(
            json["$id"].is_string(),
            "$id must be a base58 string, got {:?}",
            json["$id"]
        );
        // The key differentiator from the legacy shape: binary property as a
        // base64 string (not a u8-array). 0xdeadbeef -> "3q2+7w==".
        assert_eq!(json["blob"], serde_json::json!("3q2+7w=="));
        // Unset system fields are present as null (the legacy shape omitted them).
        assert!(
            json.get("$createdAt")
                .is_some_and(serde_json::Value::is_null),
            "unset $createdAt must be present and null, got {:?}",
            json.get("$createdAt")
        );
    }
}
