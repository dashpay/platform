//! FFI bindings for document create operations on `IdentityWallet`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dpp::document::{Document, DocumentV0Getters};
use dpp::prelude::Identifier;
use dpp::serialization::ValueConvertible;
use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::wallet::identity::crypto::tx_metadata::{
    ensure_tx_metadata_payload_fits, ensure_tx_metadata_version_supported,
};
use platform_wallet::{PlatformWalletError, TxMetadataKeySource};
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle, VTableSigner};
use zeroize::Zeroizing;

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::runtime::{block_on_worker, try_block_on_worker};
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// RAII guard scrubbing a resolved master xprv's secret scalar on drop.
/// `ExtendedPrivKey` has no `Drop`/`Zeroize` of its own, so a resolved master
/// would otherwise linger on the stack past its use — and a manual
/// `non_secure_erase()` placed after an `.await` is skipped on panic / early
/// return. Wrapping the master here scrubs it on EVERY exit path
/// Mirrors `WipingSecretKey` in `utils.rs`.
struct WipingMaster(ExtendedPrivKey);

impl Drop for WipingMaster {
    fn drop(&mut self) {
        self.0.private_key.non_secure_erase();
    }
}

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
///   The CALLER must wipe it once the derive is done — wrap it in
///   [`WipingMaster`] so its scalar is scrubbed on every exit path (normal,
///   early return, panic), not only after a manual `non_secure_erase()`. When
///   the resolver handle is null for this shape, errors with a hint naming the
///   requirement.
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
    match decide_key_source(wallet_has_resident_keys, mnemonic_resolver_handle.is_null()) {
        KeySourceDecision::ResidentWallet => Ok(None),
        KeySourceDecision::ResolverRequired => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "this wallet has no resident private keys (external-signable / watch-only); \
             a mnemonic resolver handle is required to derive its txMetadata encryption keys",
        )),
        KeySourceDecision::ResolveMaster => {
            let wallet_id = wallet.wallet_id();
            // SAFETY: handle is non-null (the decision proves it) and the
            // caller's safety contract guarantees it came from
            // `dash_sdk_mnemonic_resolver_create`.
            let master = unsafe {
                resolve_master_from_resolver(
                    mnemonic_resolver_handle,
                    &wallet_id,
                    wallet.network(),
                )?
            };
            Ok(Some(master))
        }
    }
}

/// The txMetadata payload-size policy as an FFI result.
///
/// A Rust-ABI helper, not a C symbol: the JNI layer links this crate as an
/// rlib and calls it directly, so both hosts and the C export apply one
/// implementation of the limit and cannot drift apart. It answers from the
/// declared LENGTH alone, which is what lets a caller reject an over-large
/// batch before materializing the plaintext.
///
/// Success allocates nothing; a rejection carries the core's typed explanation
/// through the same mapping every other wallet error uses.
pub fn tx_metadata_payload_len_result(payload_len: usize) -> PlatformWalletFFIResult {
    match ensure_tx_metadata_payload_fits(payload_len) {
        Ok(()) => PlatformWalletFFIResult::ok(),
        Err(error) => error.into(),
    }
}

/// Run an encrypted export's Rust-ABI inner function so a panic in it cannot
/// reach the `extern "C"` frame.
///
/// Where unwinding exists, an escaping panic would unwind into a frame declared
/// with the non-unwinding C ABI; the compiler stops that with a forced abort,
/// killing the host. Catching here turns it into an ordinary result instead.
///
/// Under `panic = "abort"` a panic aborts where it is raised, so no catch is
/// possible and the inner function is called directly. What that profile gains
/// is narrower and comes from elsewhere: the known runtime and worker-join
/// failures are handled as VALUES (see [`crate::runtime::WorkerFailure`]) and
/// so never become panics at all. Arbitrary panics remain fatal there.
///
/// The caught payload is deliberately dropped: an FFI message must stay bounded
/// and free of anything caller-derived.
fn contain_panics(inner: impl FnOnce() -> PlatformWalletFFIResult) -> PlatformWalletFFIResult {
    #[cfg(panic = "unwind")]
    {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(inner)) {
            Ok(result) => result,
            Err(_) => PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorUnknown,
                "encrypted document operation failed unexpectedly",
            ),
        }
    }
    #[cfg(not(panic = "unwind"))]
    {
        inner()
    }
}

/// Map a shared-runtime failure to an FFI result.
///
/// Neither stage is the caller's fault, so both surface as the unknown-failure
/// code carrying the failure's own fixed, stage-only text.
fn worker_failure_result(failure: crate::runtime::WorkerFailure) -> PlatformWalletFFIResult {
    PlatformWalletFFIResult::err(
        PlatformWalletFFIResultCode::ErrorUnknown,
        failure.to_string(),
    )
}

// One-shot, thread-scoped panic injection for the encrypted create inner
// function, used to prove the containment above. Consumed by the first check on
// the calling thread, leaving no state behind.
#[cfg(test)]
thread_local! {
    static FORCED_INNER_PANIC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Make the next encrypted create inner call on THIS thread panic.
#[cfg(test)]
pub(crate) fn force_inner_panic_once() {
    FORCED_INNER_PANIC.with(|flag| flag.set(true));
}

#[cfg(test)]
fn take_forced_inner_panic() {
    if FORCED_INNER_PANIC.with(|flag| flag.replace(false)) {
        panic!("forced inner panic");
    }
}

/// The key-source outcome of the capability + resolver-handle check, factored
/// out of [`tx_metadata_key_master_for_wallet`] as a pure decision so the
/// dispatch is unit-testable without a live `PlatformWallet`.
#[derive(Debug, PartialEq, Eq)]
enum KeySourceDecision {
    /// Resident-key wallet — derive in-process; the resolver handle is ignored
    /// (may be null).
    ResidentWallet,
    /// External-signable / watch-only wallet with a non-null resolver — resolve
    /// the master xprv via the host mnemonic resolver.
    ResolveMaster,
    /// External-signable / watch-only wallet but the resolver handle is null —
    /// the caller must surface the "resolver required" error.
    ResolverRequired,
}

/// Pure dispatch for [`tx_metadata_key_master_for_wallet`]: a resident-key
/// wallet always derives in-process (a null resolver handle is fine); an
/// external-signable / watch-only wallet needs the host resolver, so a null
/// handle for that shape is the "resolver required" error.
fn decide_key_source(wallet_has_resident_keys: bool, resolver_is_null: bool) -> KeySourceDecision {
    if wallet_has_resident_keys {
        KeySourceDecision::ResidentWallet
    } else if resolver_is_null {
        KeySourceDecision::ResolverRequired
    } else {
        KeySourceDecision::ResolveMaster
    }
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
/// Prepares the document synchronously via
/// `IdentityWallet::prepare_encrypted_txmetadata_properties` — the SDK selects
/// the identity's ENCRYPTION key id (the `keyIndex` field), derives the AES key
/// from the wallet HD tree, and seals the opaque `payload` into the legacy
/// `version ‖ IV ‖ AES-256-CBC` blob — then broadcasts
/// `{keyIndex, encryptionKeyIndex, encryptedMetadata}` via the generic
/// `create_document_with_signer`. The resolved master xprv is wiped BETWEEN the
/// (synchronous) derivation and the (async) broadcast, so no key material
/// crosses the network `.await`. The written document is
/// decryptable by the legacy `org.dashj.platform` stack and vice versa.
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
    // Validate the output-parameter ADDRESS and publish the documented null
    // sentinel before any other fallible input or lookup, so every later
    // rejection leaves the caller holding null rather than whatever the
    // variable happened to contain. A caller that follows the documented
    // contract would otherwise free a pointer this call never owned. This runs
    // in the `extern "C"` frame itself, so the sentinel is published even if
    // the inner function later fails in any way.
    check_ptr!(out_document_json);
    *out_document_json = ptr::null_mut();

    contain_panics(|| {
        create_encrypted_document_inner(
            wallet_handle,
            mnemonic_resolver_handle,
            owner_identity_id,
            contract_id,
            document_type_name,
            encryption_key_index,
            version,
            payload,
            payload_len,
            signer_handle,
            out_document_id,
            out_document_json,
        )
    })
}

/// Rust-ABI body of [`platform_wallet_create_encrypted_document_with_signer`].
///
/// Split out so a panic raised in here is caught before the `extern "C"` frame
/// (see [`contain_panics`]). The caller has already validated
/// `out_document_json` and published its null sentinel.
///
/// # Safety
/// Same contract as the `extern "C"` wrapper: every non-null pointer argument
/// must be valid for the duration of the call, and `out_document_json` must
/// already point to writable storage.
#[allow(clippy::too_many_arguments)]
unsafe fn create_encrypted_document_inner(
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
    #[cfg(test)]
    take_forced_inner_panic();

    check_ptr!(signer_handle);
    check_ptr!(document_type_name);
    check_ptr!(out_document_id);

    // `payload_len` is caller-supplied and bounds the copy below, so apply the
    // shared size policy to the LENGTH before the payload pointer is validated,
    // dereferenced or copied, and before any wallet, resolver or network work.
    // An over-large batch is rejectable from the arguments alone; deferring it
    // would copy the plaintext and drive a host keychain round-trip for a
    // request that can never succeed. Routed through the shared helper so this
    // path and the JNI pre-copy gate cannot diverge.
    let payload_len_check = tx_metadata_payload_len_result(payload_len);
    if payload_len_check.code != PlatformWalletFFIResultCode::Success {
        return payload_len_check;
    }

    // The wire version is likewise decidable from the argument alone. Rejecting
    // it here keeps an unsealable request from reaching the payload pointer, the
    // wallet, or the host key resolver — the last of which drives a device
    // keychain round-trip that can prompt the user.
    unwrap_result_or_return!(ensure_tx_metadata_version_supported(version));

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));
    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();

    // Copy the payload into an owned buffer. Null is allowed only for a
    // zero-length payload. It is wrapped in `Zeroizing` so the native plaintext
    // copy is scrubbed on drop, and it is dropped explicitly the instant the
    // encrypted properties are prepared (below) — the plaintext must NOT linger
    // in scope across the broadcast `.await`.
    let payload_vec: Zeroizing<Vec<u8>> = Zeroizing::new(if payload_len == 0 {
        Vec::new()
    } else {
        check_ptr!(payload);
        slice::from_raw_parts(payload, payload_len).to_vec()
    });

    let signer_addr = signer_handle as usize;
    let owner_id_for_async = owner_id;
    let contract_id_for_async = contract_id_value;

    // `move` so the closure OWNS `payload_vec` and can drop it (scrubbing the
    // plaintext) before the broadcast `.await`; the other captures are Copy or
    // already moved into the nested `async move` block.
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, move |wallet| {
        let identity_wallet = wallet.identity().clone();

        // Key-source selection by wallet capability (may synchronously call
        // back into the host mnemonic resolver for external-signable
        // wallets — see `tx_metadata_key_master_for_wallet`). The resolved
        // master is wrapped in a Drop-wiping guard.
        let master_opt =
            unsafe { tx_metadata_key_master_for_wallet(wallet, mnemonic_resolver_handle) }?
                .map(WipingMaster);

        // Derive the AES key + seal the wire blob SYNCHRONOUSLY, then wipe the
        // master BEFORE any network `.await`: the master xprv never crosses the
        // broadcast await. Only the sealed properties
        // (ciphertext, no key material) cross into the async block below.
        let key_source = match master_opt.as_ref() {
            Some(master) => TxMetadataKeySource::Master(&master.0),
            None => TxMetadataKeySource::ResidentWallet,
        };
        let properties_json = identity_wallet
            .prepare_encrypted_txmetadata_properties(
                &owner_id_for_async,
                encryption_key_index,
                version,
                &payload_vec,
                key_source,
            )
            .map_err(PlatformWalletFFIResult::from)?;
        // The plaintext is now sealed inside `properties_json` (ciphertext
        // only). Scrub the native plaintext copy AND the master immediately —
        // neither may cross the broadcast `.await` below. `payload_vec` is
        // `Zeroizing`, so the drop also wipes its bytes.
        drop(payload_vec);
        drop(master_opt);

        // Fallible worker entry: a runtime that cannot be built, or a worker
        // that does not complete, becomes a value this export maps instead of a
        // panic that would reach the C frame.
        let result: Result<(Identifier, String), PlatformWalletError> =
            try_block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                // Generic create path (no key material in scope): fetches the
                // contract, sanitizes the hex `encryptedMetadata` into `Bytes`,
                // auto-selects the AUTHENTICATION signing key, and broadcasts on
                // the 8 MB worker stack.
                let confirmed: Document = identity_wallet
                    .create_document_with_signer(
                        &owner_id_for_async,
                        &contract_id_for_async,
                        &document_type_str,
                        &properties_json,
                        signer,
                    )
                    .await?;
                let json_string = confirmed_document_to_json(&confirmed)?;
                Ok::<_, PlatformWalletError>((confirmed.id(), json_string))
            })
            .map_err(worker_failure_result)?;
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
    // Validate the output-parameter ADDRESS and publish the documented null
    // sentinel before any other fallible input or lookup, so every later
    // rejection leaves the caller holding null rather than whatever the
    // variable happened to contain. This runs in the `extern "C"` frame itself,
    // so the sentinel is published even if the inner function later fails in
    // any way.
    check_ptr!(out_documents_json);
    *out_documents_json = ptr::null_mut();

    contain_panics(|| {
        fetch_encrypted_documents_inner(
            wallet_handle,
            mnemonic_resolver_handle,
            owner_identity_id,
            contract_id,
            document_type_name,
            since_ms,
            out_documents_json,
        )
    })
}

/// Rust-ABI body of [`platform_wallet_fetch_encrypted_documents`].
///
/// Split out so a panic raised in here is caught before the `extern "C"` frame
/// (see [`contain_panics`]). The caller has already validated
/// `out_documents_json` and published its null sentinel.
///
/// # Safety
/// Same contract as the `extern "C"` wrapper: every non-null pointer argument
/// must be valid for the duration of the call, and `out_documents_json` must
/// already point to writable storage.
unsafe fn fetch_encrypted_documents_inner(
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
        // wallets — see `tx_metadata_key_master_for_wallet`). The resolved
        // master is wrapped in a Drop-wiping guard.
        let master_opt =
            unsafe { tx_metadata_key_master_for_wallet(wallet, mnemonic_resolver_handle) }?
                .map(WipingMaster);

        // Fallible worker entry: a runtime that cannot be built, or a worker
        // that does not complete, becomes a value this export maps instead of a
        // panic that would reach the C frame.
        let result: Result<Vec<platform_wallet::DecryptedEncryptedDocument>, PlatformWalletError> =
            try_block_on_worker(async move {
                // TRADEOFF: unlike create, a document's
                // (keyIndex, encryptionKeyIndex) are only known AFTER its page is
                // fetched, so the master cannot be fully pre-derived before the
                // network work. It therefore stays resident across the pagination
                // awaits — but inside the `WipingMaster` Drop guard, so a panic or
                // early return still scrubs its scalar (a manual post-await erase
                // would be skipped on those paths). Per-document key derivation is
                // itself synchronous, between page fetches (see
                // `fetch_encrypted_documents`).
                let key_source = match master_opt.as_ref() {
                    Some(master) => TxMetadataKeySource::Master(&master.0),
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
                drop(master_opt); // scrub as soon as the fetch completes
                fetched
            })
            .map_err(worker_failure_result)?;
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
    // The single source of truth for the limit, so the boundary tests cannot
    // drift from the Rust core that enforces it.
    use platform_wallet::wallet::identity::crypto::tx_metadata::MAX_TX_METADATA_PLAINTEXT_LEN;
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

    // ── tx_metadata_key_master_for_wallet dispatch ──────────────────────────
    //
    // `tx_metadata_key_master_for_wallet` needs a live `PlatformWallet` (wallet
    // manager + SDK), which a unit test can't cheaply build, so its load-bearing
    // branch logic is factored into the pure `decide_key_source`. These pin the
    // capability dispatch, the null-handle handling, and the resolver-required
    // error path that the FFI create/fetch entry points rely on.

    /// A resident-key wallet derives in-process — the resolver handle is
    /// irrelevant, so a NULL handle is fine (never the "resolver required" error).
    #[test]
    fn resident_wallet_ignores_resolver_handle_even_when_null() {
        assert_eq!(
            decide_key_source(true, true),
            KeySourceDecision::ResidentWallet,
            "resident wallet + null resolver must derive in-process, not error"
        );
        assert_eq!(
            decide_key_source(true, false),
            KeySourceDecision::ResidentWallet,
            "resident wallet + non-null resolver still derives in-process"
        );
    }

    /// An external-signable / watch-only wallet dispatches to the resolver-master
    /// path when a (non-null) resolver handle is supplied.
    #[test]
    fn external_signable_wallet_dispatches_to_resolver_master() {
        assert_eq!(
            decide_key_source(false, false),
            KeySourceDecision::ResolveMaster,
            "external-signable / watch-only wallet + resolver must resolve the master"
        );
    }

    /// An external-signable / watch-only wallet with a NULL resolver handle is
    /// the "resolver required" error path (the on-device shape that must not
    /// silently derive the wrong key).
    #[test]
    fn external_signable_wallet_null_resolver_is_resolver_required() {
        assert_eq!(
            decide_key_source(false, true),
            KeySourceDecision::ResolverRequired,
            "external-signable / watch-only wallet + null resolver must error, not derive"
        );
    }

    // ── Encrypted-export input contracts ────────────────────────────────────
    //
    // These drive the real `extern "C"` entry points. They deliberately use a
    // wallet handle that is absent from `PLATFORM_WALLET_STORAGE`: the handle
    // lookup is a plain map `get`, so an unknown handle is a safe, ordinary
    // miss (`NotFound`) and NO closure body — resolver callback, key
    // derivation, allocator, or broadcast — ever runs. That miss is what makes
    // the ordering observable: whichever check reports first is the check that
    // ran first.

    /// A wallet handle guaranteed absent from the storage map, so the export's
    /// `with_item` closure cannot run.
    const UNKNOWN_WALLET_HANDLE: Handle = u64::MAX;

    /// A non-null pointer to real, test-owned storage, used where the export
    /// only checks for null and never dereferences.
    fn opaque_non_null<T>(storage: &mut u8) -> *mut T {
        storage as *mut u8 as *mut T
    }

    // ── Resolver dispatch through the encrypted create export ───────────────
    //
    // A wallet registered through `PlatformWalletManager` is downgraded to
    // external-signable before it is stored, so it holds no in-process private
    // keys and its txMetadata key must come from the host mnemonic resolver.
    // That makes the real export drive a real resolver vtable, with the mock
    // SDK guaranteeing no network is reachable.

    /// Host-side resolver context: the phrase to hand back, plus a count of how
    /// many times the host was consulted.
    struct ResolverContext {
        /// Derived at runtime from all-zero entropy so no recovery phrase is
        /// committed to the repository.
        phrase: String,
        calls: std::sync::atomic::AtomicUsize,
    }

    /// Resolver callback that answers with the generated mnemonic and records
    /// that the host was consulted — the observable that says whether the
    /// export reached out to the device keychain.
    unsafe extern "C" fn counting_resolve(
        ctx: *const std::ffi::c_void,
        _wallet_id_bytes: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        let context = &*(ctx as *const ResolverContext);
        context
            .calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let phrase = context.phrase.as_bytes();
        if phrase.len() + 1 > out_capacity {
            return rs_sdk_ffi::mnemonic_resolver_result::BUFFER_TOO_SMALL;
        }
        ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_buf.add(phrase.len()) = 0;
        *out_len = phrase.len();
        rs_sdk_ffi::mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn noop_destroy(_ctx: *mut std::ffi::c_void) {}

    /// The identity the fixture owns, and the id the export is called with.
    const FIXTURE_OWNER: [u8; 32] = [3u8; 32];

    /// A live wallet handle backed by a mock SDK, plus the resolver handle and
    /// its shared context.
    struct ResolverFixture {
        wallet_handle: Handle,
        resolver: *mut MnemonicResolverHandle,
        context: *mut ResolverContext,
        manager_handle: Handle,
        _sdk: Box<dash_sdk::Sdk>,
    }

    impl ResolverFixture {
        fn resolver_calls(&self) -> usize {
            unsafe {
                (*self.context)
                    .calls
                    .load(std::sync::atomic::Ordering::SeqCst)
            }
        }
    }

    impl Drop for ResolverFixture {
        fn drop(&mut self) {
            unsafe {
                // Release the wallet handle BEFORE the manager so the process-
                // wide `PLATFORM_WALLET_STORAGE` does not accumulate a wallet
                // per test run.
                let _ = crate::wallet::platform_wallet_destroy(self.wallet_handle);
                let _ = crate::manager::platform_wallet_manager_destroy(self.manager_handle);
                // `noop_destroy` frees nothing, so the context is owned here and
                // freed exactly once.
                rs_sdk_ffi::dash_sdk_mnemonic_resolver_destroy(self.resolver);
                drop(Box::from_raw(self.context));
            }
        }
    }

    /// An identity carrying the ECDSA key the txMetadata key derivation selects
    /// (`AUTHENTICATION` / `HIGH`, the documented fallback arm).
    fn fixture_identity() -> dpp::identity::Identity {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::v0::IdentityV0;
        use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};

        let key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 2,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(vec![0x02; 33]),
            disabled_at: None,
        });
        let mut public_keys = BTreeMap::new();
        public_keys.insert(2, key);

        dpp::identity::Identity::V0(IdentityV0 {
            id: Identifier::from(FIXTURE_OWNER),
            public_keys,
            balance: 0,
            revision: 0,
        })
    }

    /// Build a manager on a mock SDK, register a wallet through the real FFI
    /// path (which stores it as external-signable), give it a resident identity
    /// slot so key preparation can proceed, and wire a counting host resolver.
    fn resolver_fixture() -> ResolverFixture {
        use key_wallet::mnemonic::{Language, Mnemonic};
        use std::ffi::c_void;

        unsafe extern "C" fn begin_changeset(_ctx: *mut c_void, _wallet_id: *const u8) -> i32 {
            0
        }
        unsafe extern "C" fn end_changeset(
            _ctx: *mut c_void,
            _wallet_id: *const u8,
            _success: bool,
        ) -> i32 {
            0
        }

        // Generated, never committed.
        let mnemonic =
            Mnemonic::from_entropy(&[0u8; 16], Language::English).expect("16 bytes of entropy");
        let phrase = mnemonic.phrase().to_string();

        let sdk = Box::new(
            dash_sdk::SdkBuilder::new_mock()
                .build()
                .expect("mock sdk builds"),
        );
        let persistence = crate::PersistenceCallbacks {
            on_changeset_begin_fn: Some(begin_changeset),
            on_changeset_end_fn: Some(end_changeset),
            ..Default::default()
        };
        let events = crate::EventHandlerCallbacks {
            context: ptr::null_mut(),
            on_wallet_event_fn: None,
            on_error_fn: None,
            on_platform_address_sync_completed_fn: None,
            on_shielded_sync_completed_fn: None,
            on_shielded_sync_progress_fn: None,
            on_shielded_tree_progress_fn: None,
        };

        let mut manager_handle: Handle = 0;
        let result = unsafe {
            crate::manager::platform_wallet_manager_create(
                &*sdk as *const dash_sdk::Sdk as *const c_void,
                &persistence,
                &events,
                &mut manager_handle,
            )
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::Success,
            "manager creation must succeed"
        );

        let mnemonic_c = CString::new(phrase.clone()).expect("no interior NUL");
        let mut wallet_handle: Handle = 0;
        let mut wallet_id = [0u8; 32];
        let result = unsafe {
            crate::manager::platform_wallet_manager_create_wallet_from_mnemonic(
                manager_handle,
                mnemonic_c.as_ptr(),
                crate::FFINetwork::Testnet,
                0, // no accounts: nothing here needs an address pool
                &mut wallet_handle,
                &mut wallet_id,
            )
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::Success,
            "wallet registration must succeed on a mock SDK"
        );

        // Give the wallet a resident identity slot so key preparation resolves
        // an encryption context and reaches the wire-version check.
        PLATFORM_WALLET_STORAGE
            .with_item(wallet_handle, |wallet| {
                let persister = wallet.persister().clone();
                let id = wallet.wallet_id();
                let mut wm = wallet.wallet_manager().blocking_write();
                let info = wm.get_wallet_info_mut(&id).expect("registered wallet info");
                info.identity_manager
                    .add_identity(fixture_identity(), 0, id, &persister)
                    .expect("add the fixture identity");
            })
            .expect("wallet handle is live");

        let context = Box::into_raw(Box::new(ResolverContext {
            phrase,
            calls: std::sync::atomic::AtomicUsize::new(0),
        }));
        let resolver = unsafe {
            rs_sdk_ffi::dash_sdk_mnemonic_resolver_create(
                context as *mut std::ffi::c_void,
                counting_resolve,
                noop_destroy,
            )
        };

        ResolverFixture {
            wallet_handle,
            resolver,
            context,
            manager_handle,
            _sdk: sdk,
        }
    }

    /// Invoke the encrypted create export against the fixture wallet, returning
    /// the result and whether the JSON out-pointer was left null.
    fn create_encrypted_with(
        fixture: &ResolverFixture,
        version: u8,
    ) -> (PlatformWalletFFIResult, bool) {
        let mut out_json: *mut c_char = ptr::null_mut();
        let mut out_id = [0u8; 32];
        let doc_type = CString::new("txMetadata").expect("no interior NUL");
        let contract = [4u8; 32];
        let payload = [0xabu8; 16];
        let mut signer_storage = 0u8;

        let result = unsafe {
            platform_wallet_create_encrypted_document_with_signer(
                fixture.wallet_handle,
                fixture.resolver,
                FIXTURE_OWNER.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                0,
                version,
                payload.as_ptr(),
                payload.len(),
                opaque_non_null(&mut signer_storage),
                out_id.as_mut_ptr(),
                &mut out_json,
            )
        };

        let json_is_null = out_json.is_null();
        // Defensive: the documented contract is that this stays null on every
        // error path, but reclaim it rather than leak if that ever changes.
        if !json_is_null {
            unsafe { drop(CString::from_raw(out_json)) };
        }
        (result, json_is_null)
    }

    /// Invoke the encrypted fetch export against the fixture wallet, returning
    /// the result and whether the JSON out-pointer was left null.
    fn fetch_encrypted_with(fixture: &ResolverFixture) -> (PlatformWalletFFIResult, bool) {
        let mut out_json: *mut c_char = ptr::null_mut();
        let doc_type = CString::new("txMetadata").expect("no interior NUL");
        let contract = [4u8; 32];

        let result = unsafe {
            platform_wallet_fetch_encrypted_documents(
                fixture.wallet_handle,
                fixture.resolver,
                FIXTURE_OWNER.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                0,
                &mut out_json,
            )
        };

        let json_is_null = out_json.is_null();
        if !json_is_null {
            unsafe { drop(CString::from_raw(out_json)) };
        }
        (result, json_is_null)
    }

    /// The encrypted create export, driven end to end on an external-signable
    /// wallet with a real host resolver vtable.
    ///
    /// A wire version the core cannot encode is decidable from the argument
    /// alone, so it must be rejected before the export touches the payload
    /// pointer, the wallet, the host resolver or the network. Reaching the
    /// resolver first drives a device keychain round-trip — a user-visible
    /// prompt on some hosts — for a request that can never succeed.
    #[test]
    fn create_encrypted_rejects_an_unsupported_version_before_resolver_or_key_work() {
        let fixture = resolver_fixture();

        let (result, json_is_null) = create_encrypted_with(&fixture, 2);

        assert_eq!(
            fixture.resolver_calls(),
            0,
            "an unsupported version is decidable from the arguments, so the host \
             resolver must never be consulted for it"
        );
        assert!(
            json_is_null,
            "the JSON out-pointer must remain null on this error path"
        );
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "an unsupported wire version must reach the host as a distinguishable, \
             Rust-owned caller-input error; flattening it leaves hosts unable to tell \
             it from any other failure, which is why they still carry their own \
             version literals"
        );
    }

    // ── Runtime / worker failure containment at the C boundary ──────────────
    //
    // Both encrypted exports drive async work through the shared runtime. A
    // runtime that fails to build, and a worker future that panics, are the two
    // ways that machinery fails independently of the request. Left as panics,
    // both end the host process: on an unwinding profile the unwind reaches the
    // non-unwinding `extern "C"` frame and is stopped there by a forced abort;
    // on an aborting profile the panic aborts where it is raised. Each must
    // instead become an ordinary FFI result, with the documented nullable output
    // still null.
    //
    // What that buys differs by profile. Runtime-construction failure, and a
    // join error the runtime reports explicitly, become values on any profile.
    // Recovering from a worker that PANICKED, and catching a panic raised in the
    // inner function, both require unwinding; under `panic = "abort"` the panic
    // aborts at its origin and neither mapping runs. The forced-failure tests
    // below therefore cover the value paths, while the inner-panic test is
    // gated to unwind builds.
    //
    // The forced failures are one-shot and scoped to the calling thread, so
    // these tests do not perturb anything running in parallel.

    /// A worker that fails to join must reach the host as an ordinary error.
    #[test]
    fn create_encrypted_maps_forced_worker_join_failure_to_error_unknown_and_keeps_json_null() {
        let fixture = resolver_fixture();
        crate::runtime::force_worker_join_failure_once();

        // Version 1 is supported, so the request passes validation and key
        // preparation and reaches the worker gate, where the failure is forced.
        let (result, json_is_null) = create_encrypted_with(&fixture, 1);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUnknown,
            "a worker join failure is not a caller-input error; it must surface as \
             the unknown-failure code rather than ending the host process"
        );
        assert!(
            json_is_null,
            "the JSON out-pointer must remain null when the worker fails"
        );
        // The supported-version counterpart of the early version gate: a
        // version the core accepts must still reach the host resolver, so the
        // gate cannot be satisfied by rejecting everything.
        assert_eq!(
            fixture.resolver_calls(),
            1,
            "a supported version must proceed to derive its key through the host \
             resolver exactly once"
        );
    }

    /// The same containment on the fetch export.
    #[test]
    fn fetch_encrypted_maps_forced_worker_join_failure_to_error_unknown_and_keeps_json_null() {
        let fixture = resolver_fixture();
        crate::runtime::force_worker_join_failure_once();

        let (result, json_is_null) = fetch_encrypted_with(&fixture);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUnknown,
            "a worker join failure on the fetch path must surface as the \
             unknown-failure code rather than ending the host process"
        );
        assert!(
            json_is_null,
            "the JSON out-pointer must remain null when the worker fails"
        );
    }

    /// Runtime construction failure is the other independent machinery failure.
    /// One export is enough: both reach it through the same shared helper.
    #[test]
    fn create_encrypted_maps_forced_runtime_init_failure_to_error_unknown_and_keeps_json_null() {
        let fixture = resolver_fixture();
        crate::runtime::force_runtime_init_failure_once();

        let (result, json_is_null) = create_encrypted_with(&fixture, 1);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUnknown,
            "a runtime that cannot be built must surface as the unknown-failure \
             code rather than aborting the host"
        );
        assert!(
            json_is_null,
            "the JSON out-pointer must remain null when the runtime cannot be built"
        );
    }

    /// Where unwinding exists, a panic raised inside the export's Rust-ABI inner
    /// function must be caught before the `extern "C"` frame — otherwise the
    /// unwind reaches a frame that cannot unwind and is turned into a forced
    /// abort. On an aborting profile the panic aborts at its origin, so no catch
    /// can help and the guarantee is asserted only where it can hold.
    #[cfg(panic = "unwind")]
    #[test]
    fn create_encrypted_contains_an_inner_panic_before_the_extern_c_boundary() {
        let fixture = resolver_fixture();
        force_inner_panic_once();

        let (result, json_is_null) = create_encrypted_with(&fixture, 1);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUnknown,
            "a panic inside the inner function must be converted to a result before \
             the C frame, rather than unwinding into it and forcing an abort"
        );
        assert!(
            json_is_null,
            "the JSON out-pointer must remain null when the inner function panics"
        );
    }

    // Both encrypted exports document that `*out_..._json` is "left null" on
    // ANY error, so a host reads that pointer after a failure and frees it when
    // non-null. The null sentinel must therefore be published BEFORE any other
    // fallible input validation — otherwise an early rejection returns with the
    // caller's variable still holding its prior value, and a host following the
    // documented contract frees a pointer this call never owned.

    /// An early input rejection must still leave `*out_document_json` null.
    #[test]
    fn create_encrypted_publishes_null_json_out_before_other_validation() {
        // Real, test-owned storage so the sentinel is a valid non-null pointer
        // rather than a fabricated address.
        let mut sentinel_storage: c_char = 0x7f;
        let mut out_json: *mut c_char = &mut sentinel_storage;
        let mut out_id = [0u8; 32];
        let doc_type = CString::new("txMetadata").expect("no interior NUL");
        let owner = [1u8; 32];
        let contract = [2u8; 32];
        let payload = [0xabu8; 8];

        // A NULL signer handle: a deliberately invalid input that trips an
        // early validation path. The documented null-output contract must hold
        // there too, not only on failures reached later in the call.
        let result = unsafe {
            platform_wallet_create_encrypted_document_with_signer(
                UNKNOWN_WALLET_HANDLE,
                ptr::null_mut(),
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                0,
                1,
                payload.as_ptr(),
                payload.len(),
                ptr::null_mut(),
                out_id.as_mut_ptr(),
                &mut out_json,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert!(
            out_json.is_null(),
            "the export documents `*out_document_json` as null on any error, but an \
             early input rejection returned with the caller's pointer unchanged"
        );
    }

    /// Same contract on the fetch export's documented nullable output.
    #[test]
    fn fetch_encrypted_publishes_null_json_out_before_other_validation() {
        let mut sentinel_storage: c_char = 0x7f;
        let mut out_json: *mut c_char = &mut sentinel_storage;
        let owner = [1u8; 32];
        let contract = [2u8; 32];

        // A NULL document type: a deliberately invalid input that trips an
        // early validation path. The documented null-output contract must hold
        // there too, not only on failures reached later in the call.
        let result = unsafe {
            platform_wallet_fetch_encrypted_documents(
                UNKNOWN_WALLET_HANDLE,
                ptr::null_mut(),
                owner.as_ptr(),
                contract.as_ptr(),
                ptr::null(),
                0,
                &mut out_json,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert!(
            out_json.is_null(),
            "the export documents `*out_documents_json` as null on any error, but an \
             early input rejection returned with the caller's pointer unchanged"
        );
    }

    // The payload length limit is enforced by the Rust core; the boundary must
    // apply it before it touches the payload pointer at all. `payload_len` is
    // caller-supplied and is used both as the copy length and as the value the
    // limit applies to, so checking the length first is what keeps an oversized
    // or mis-declared length from reaching a dereference, a native copy, the
    // wallet lookup, or the host resolver callback.

    /// A NULL payload pointer with an over-limit declared length: the length is
    /// rejectable without reading a single byte. Returning the size error here
    /// proves the limit is applied BEFORE pointer validation, dereference and
    /// the native copy — the same gate that keeps an oversized real buffer from
    /// being copied and from driving the resolver.
    #[test]
    fn create_encrypted_rejects_oversized_length_before_touching_the_payload_pointer() {
        let mut sentinel_storage: c_char = 0x7f;
        let mut out_json: *mut c_char = &mut sentinel_storage;
        let mut out_id = [0u8; 32];
        let doc_type = CString::new("txMetadata").expect("no interior NUL");
        let owner = [1u8; 32];
        let contract = [2u8; 32];
        let mut signer_storage = 0u8;
        let oversized_len = MAX_TX_METADATA_PLAINTEXT_LEN + 1;

        let result = unsafe {
            platform_wallet_create_encrypted_document_with_signer(
                UNKNOWN_WALLET_HANDLE,
                ptr::null_mut(),
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                0,
                1,
                // NULL payload: nothing here may legally be read, so any answer
                // other than the size error means the length was not consulted
                // first.
                ptr::null(),
                oversized_len,
                opaque_non_null(&mut signer_storage),
                out_id.as_mut_ptr(),
                &mut out_json,
            )
        };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "a declared length of {oversized_len} exceeds the \
             {MAX_TX_METADATA_PLAINTEXT_LEN}-byte maximum and is rejectable without \
             reading the payload; reporting any other failure first proves the \
             boundary validates and dereferences the pointer, copies the plaintext, \
             and enters the wallet/resolver path before the length is ever checked"
        );
    }

    /// The accepted side of the same boundary, driven with a REAL buffer: a
    /// maximum-length payload must pass the size gate and fail only at the
    /// (absent) wallet handle, proving the gate rejects the over-limit length
    /// without also rejecting the largest legal one.
    #[test]
    fn create_encrypted_accepts_maximum_payload_at_the_size_gate() {
        let mut sentinel_storage: c_char = 0x7f;
        let mut out_json: *mut c_char = &mut sentinel_storage;
        let mut out_id = [0u8; 32];
        let doc_type = CString::new("txMetadata").expect("no interior NUL");
        let owner = [1u8; 32];
        let contract = [2u8; 32];
        let payload = vec![0xabu8; MAX_TX_METADATA_PLAINTEXT_LEN];
        let mut signer_storage = 0u8;

        let result = unsafe {
            platform_wallet_create_encrypted_document_with_signer(
                UNKNOWN_WALLET_HANDLE,
                ptr::null_mut(),
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                0,
                1,
                payload.as_ptr(),
                payload.len(),
                opaque_non_null(&mut signer_storage),
                out_id.as_mut_ptr(),
                &mut out_json,
            )
        };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::NotFound,
            "a {MAX_TX_METADATA_PLAINTEXT_LEN}-byte payload is exactly the maximum \
             and must pass the size gate, reaching the wallet-handle lookup"
        );
    }
}
