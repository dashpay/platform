//! FFI bindings for document create operations on `IdentityWallet`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dpp::document::serialization_traits::DocumentJsonMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use platform_wallet::PlatformWalletError;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

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
/// is the canonical query-side representation — produced via DPP's
/// `Document::to_json_with_identifiers_using_bytes`, so it carries the
/// system fields (`$id`/`$ownerId`, set timestamps, `$revision`) with
/// identifiers rendered as base58 strings and only the document's
/// populated fields present (DPP-normalized + default-filled). Swift
/// persists this body verbatim so the local cache matches what a
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
                // Serialize the confirmed document to its canonical
                // query-side JSON. `to_json_with_identifiers_using_bytes`
                // renders `$id`/`$ownerId` (and `$creatorId`) as base58
                // strings and emits only the populated system fields — the
                // shape a DOC-01 query display expects. The trait's
                // `platform_version` argument is unused by the V0 impl, so
                // `latest()` (the same convention the other FFI entry points
                // in this crate use) is safe here.
                let platform_version = PlatformVersion::latest();
                let json_value = confirmed
                    .to_json_with_identifiers_using_bytes(platform_version)
                    .map_err(|e| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "Failed to convert confirmed document to JSON: {e}"
                        ))
                    })?;
                let json_string = serde_json::to_string(&json_value).map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to serialize confirmed document JSON: {e}"
                    ))
                })?;
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
