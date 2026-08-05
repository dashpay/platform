//! FFI bindings for document create operations on `IdentityWallet`.

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dpp::document::{Document, DocumentV0Getters};
use dpp::prelude::Identifier;
use dpp::serialization::ValueConvertible;
use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::wallet::identity::crypto::tx_metadata::ensure_tx_metadata_create_inputs_valid;
use platform_wallet::{PlatformWalletError, TxMetadataKeySource};
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle, VTableSigner};
use zeroize::Zeroizing;

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::runtime::{block_on_worker, try_block_on_worker};
use crate::tx_metadata_json::serialize_decrypted_documents;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// RAII guard scrubbing a resolved master xprv's secret scalar on drop.
///
/// The pinned `ExtendedPrivKey` zeroizes itself on drop. This guard narrows the
/// private scalar's lifetime to the explicit operation boundary and keeps that
/// boundary stable across later lexical refactors: ordinary return, error
/// return, and unwinding panic all erase it here before the value's own full
/// zeroizing drop runs.
///
/// It does NOT cover `panic = "abort"`, which the iOS profiles use: an abort
/// runs no destructor, so nothing scrubs the master there. Nor is the write
/// itself absolute — it cannot reach a register copy or one the optimizer
/// already made. Mirrors `WipingSecretKey` in `utils.rs`.
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
///   [`WipingMaster`] so its scalar is scrubbed on ordinary return, on an error
///   return and on an unwinding panic, rather than only after a manual
///   `non_secure_erase()`. An abort runs no destructor and is not covered. When
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

/// The whole txMetadata create-argument policy as an FFI result.
///
/// A Rust helper, not a C symbol, shared by the C exports, deferred-payload
/// composite, and their tests so every entry point applies one implementation
/// of what makes a create request valid.
///
/// Every caller runs this BEFORE copying plaintext, consulting the host key
/// resolver, reaching the network, or reserving an index — including the index
/// allocation path, which must not spend an index on a request that a later stage
/// will reject anyway. `encryption_key_index` is `None` when an index is about to
/// be allocated.
///
/// `signer_present` carries the one precondition that is not wallet-protocol
/// policy: a create broadcasts through a signer, so a request without one cannot
/// succeed no matter what the other arguments say. It lives here rather than
/// only at a C wrapper so the C exports and Rust-ABI deferred-payload composite
/// all reject the same requests before materializing plaintext or reserving an
/// index.
pub fn tx_metadata_create_preflight_result(
    payload_len: usize,
    version: u8,
    encryption_key_index: Option<u32>,
    signer_present: bool,
) -> PlatformWalletFFIResult {
    if !signer_present {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorNullPointer,
            "signer_handle ptr is null",
        );
    }
    match ensure_tx_metadata_create_inputs_valid(payload_len, version, encryption_key_index) {
        Ok(()) => PlatformWalletFFIResult::ok(),
        Err(error) => error.into(),
    }
}

/// Sequence an encrypted create's index and complete encryption preparation
/// ahead of its plaintext copy.
///
/// Resolving the index can wait on the network (the SDK-allocated path counts
/// the identity's documents on Platform) and needs only the payload's length,
/// while materializing produces an owned copy of the caller's plaintext. Running
/// them in this order is what keeps a native plaintext copy from existing while
/// that round trip, a host-backed key lookup, managed-owner resolution, key
/// selection, or AES derivation is in flight. A failed preparation returns
/// before `materialize` runs at all, so a doomed request never copies the
/// plaintext either.
///
/// The order is expressed as a function rather than as adjacent statements
/// because it is a security property, not a stylistic one: a later edit that
/// reorders it has to change this call, and the ordering test pinning it.
///
/// This private seam is used by the shared create orchestration for both
/// borrowed C input and deferred host materialization, so neither host bridge
/// decides the ordering. It resolves the index, prepares a zeroizing encryption
/// context, materializes the native plaintext exactly once, and verifies that
/// the callback honored the declared-length contract.
fn settle_index_prepare_encryption_and_materialize_payload<Secret>(
    declared_len: usize,
    resolve_index: impl FnOnce() -> Result<u32, PlatformWalletFFIResult>,
    prepare: impl FnOnce(u32) -> Result<Secret, PlatformWalletFFIResult>,
    materialize: impl FnOnce() -> Result<Zeroizing<Vec<u8>>, PlatformWalletFFIResult>,
) -> Result<(u32, Secret, Zeroizing<Vec<u8>>), PlatformWalletFFIResult> {
    let resolved_index = resolve_index()?;
    let prepared = prepare(resolved_index)?;
    let payload = materialize()?;
    if payload.len() != declared_len {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!(
                "materialized payload length {} did not match declared length {declared_len}",
                payload.len()
            ),
        ));
    }
    Ok((resolved_index, prepared, payload))
}

/// Seal the plaintext, release every SDK-owned secret, and only THEN broadcast.
///
/// Broadcasting is an unbounded network wait — the SDK sets no request timeout —
/// so whatever is still alive when it starts stays alive for however long the
/// network takes. Sealing needs the plaintext and the resolved key material;
/// broadcasting needs neither, only the ciphertext-bearing properties. Ending
/// both lifetimes strictly between the two stages is what keeps them off that
/// wait, and a seal that fails releases them without reaching the network at
/// all.
///
/// The order is expressed as a function rather than as adjacent statements
/// because it is a security property, not a stylistic one: a later edit that
/// reorders it has to change this call, and the ordering tests pinning it.
///
/// It is also what carries the guarantee across the SDK's own boundary. A host
/// that cannot lend its plaintext hands over a bridge-created native copy
/// instead; that copy becomes `payload` here, so the release below erases that
/// native copy rather than merely erasing a second copy made inside the SDK.
/// Any runtime-managed host object remains outside Rust's control.
fn seal_and_release_before_broadcasting<Payload, Secret, Sealed, Broadcast, Failure>(
    payload: Payload,
    secret: Secret,
    seal: impl FnOnce(&Payload, &Secret) -> Result<Sealed, Failure>,
    broadcast: impl FnOnce(Sealed) -> Broadcast,
) -> Result<Broadcast, Failure> {
    // Released explicitly on BOTH paths rather than left to end-of-scope, so
    // the point at which each lifetime ends is stated here rather than implied
    // by declaration order.
    let sealed = match seal(&payload, &secret) {
        Ok(sealed) => sealed,
        Err(failure) => {
            drop(payload);
            drop(secret);
            return Err(failure);
        }
    };
    drop(payload);
    drop(secret);
    Ok(broadcast(sealed))
}

/// Where an encrypted create's plaintext comes from and — inseparably — how its
/// `encryptionKeyIndex` was settled.
///
/// The two shapes exist because hosts differ in what they can lend. A host that
/// owns its plaintext outright lends a pointer to it for the synchronous call
/// (Swift's `Data.withUnsafeBytes`), so the SDK copies it internally and is free
/// to settle the index itself first. A host whose plaintext lives in a
/// runtime-managed object that cannot be pinned across a network round trip (a
/// JVM `byte[]`) instead gives Rust a deferred materializer. Rust settles the
/// index first and invokes that callback only when it is ready to take ownership
/// of the native plaintext copy.
///
/// Pairing the declared length, optional index, and materializer keeps the
/// ordering in this shared Rust operation rather than in either host bridge.
type DeferredPayloadMaterializer<'a> =
    Box<dyn FnOnce() -> Result<Zeroizing<Vec<u8>>, PlatformWalletFFIResult> + 'a>;

enum PayloadSource<'a> {
    /// Caller memory borrowed for the synchronous call, copied into an owned
    /// zeroizing buffer only once the index and complete encryption context
    /// are prepared.
    Borrowed {
        ptr: *const u8,
        len: usize,
        index: Option<u32>,
        borrow: PhantomData<&'a [u8]>,
    },
    /// A native copy that Rust asks the host bridge to make only after the
    /// index and complete encryption context are prepared. The callback runs
    /// synchronously on the thread that entered this Rust-ABI helper and
    /// returns ownership of the copy.
    Deferred {
        len: usize,
        index: Option<u32>,
        materialize: DeferredPayloadMaterializer<'a>,
    },
}

impl PayloadSource<'_> {
    /// The plaintext length. On the borrowed shape this is the DECLARED length,
    /// which is what lets an over-large request be refused without the pointer
    /// ever being read.
    fn len(&self) -> usize {
        match self {
            PayloadSource::Borrowed { len, .. } => *len,
            PayloadSource::Deferred { len, .. } => *len,
        }
    }

    /// The caller-supplied index, or `None` when the SDK is about to allocate
    /// one before materialization.
    fn settled_index(&self) -> Option<u32> {
        match self {
            PayloadSource::Borrowed { index, .. } => *index,
            PayloadSource::Deferred { index, .. } => *index,
        }
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

#[cfg(not(test))]
fn take_forced_inner_panic() {}

/// The key-source outcome of the capability + resolver-handle check, factored
/// out of [`tx_metadata_key_master_for_wallet`] as a pure decision so the
/// dispatch is decidable without a live `PlatformWallet`.
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
/// `IdentityWallet::prepare_txmetadata_encryption_blocking` — the SDK resolves the
/// identity, selects its ENCRYPTION key id (the `keyIndex` field), and derives
/// the AES key from the wallet HD tree before the host payload is copied. It
/// then seals the opaque `payload` into the legacy
/// `version ‖ IV ‖ AES-256-CBC` blob and broadcasts
/// `{keyIndex, encryptionKeyIndex, encryptedMetadata}` via the generic
/// `create_document_with_signer`. The resolved master xprv is wiped BEFORE the
/// host payload is copied, and the derived key is dropped before the async
/// broadcast, so no key material crosses the network `.await`. The written
/// document is decryptable by the legacy `org.dashj.platform` stack and vice
/// versa.
///
/// The AES key source is selected by the wallet's capability: a key-resident
/// wallet derives in-process; an external-signable / watch-only wallet (the
/// Android/iOS apps) derives through `mnemonic_resolver_handle` — required
/// non-null for that shape, ignored otherwise (see
/// `tx_metadata_key_master_for_wallet`).
///
/// The caller supplies `encryption_key_index` (the app's per-document index),
/// `version` (`1` = protobuf, as the wallet writes), and the already-serialized
/// opaque `payload` (a protobuf `TxMetadataBatch`; the SDK does not parse it).
/// `payload` may be null only when `payload_len == 0`.
///
/// This explicit-index entry point is retained for migration / tests. New
/// hosts should prefer the ABI-additive sibling
/// [`platform_wallet_create_encrypted_document_with_signer_auto_index`], which
/// omits `encryption_key_index` and lets Rust allocate it from authoritative
/// Platform state — moving the index-selection
/// policy off the host.
///
/// On success the confirmed document's 32-byte id is written to
/// `out_document_id` and its canonical query-side JSON to `*out_document_json`
/// (release with `platform_wallet_string_free`; left null on any error).
///
/// # Safety
/// Every pointer below must stay valid for the whole synchronous duration of
/// this call; the call borrows them and retains none of them afterwards.
///
/// - `owner_identity_id` and `contract_id` must each point to 32 readable bytes.
/// - `document_type_name` must be a valid NUL-terminated C string of UTF-8.
/// - `payload` and `payload_len` are one unit: `payload` must point to
///   `payload_len` readable bytes. `payload` may be null ONLY when
///   `payload_len == 0`; a null pointer with a non-zero length is rejected from
///   the arguments and never dereferenced. The bytes are copied into an owned
///   zeroizing buffer, so the caller may free or overwrite its own buffer as
///   soon as this returns — the caller's buffer is never scrubbed by the SDK.
/// - `signer_handle` must be a live handle for the duration of the call and must
///   not be null; a create cannot broadcast without one. `mnemonic_resolver_handle`
///   may be null for a wallet with resident private keys, and must be live and
///   non-null for an external-signable wallet.
/// - `out_document_id` must point to 32 writable bytes. It is written only on
///   success.
/// - `out_document_json` must point to writable storage for one `char *`. It is
///   set to null before any other fallible work, so on EVERY error path the
///   caller is left holding null and must free nothing. On success it receives
///   ownership of a NUL-terminated C string that the caller MUST release with
///   `platform_wallet_string_free` — the ordinary free. This output is canonical
///   document JSON (ciphertext and metadata, no plaintext); do NOT pass it to
///   `platform_wallet_sensitive_string_free`, which pairs with the fetch
///   export's output instead.
///
/// The returned `PlatformWalletFFIResult` owns its message and must be released
/// with `platform_wallet_ffi_result_free`.
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
            version,
            PayloadSource::Borrowed {
                ptr: payload,
                len: payload_len,
                index: Some(encryption_key_index),
                borrow: PhantomData,
            },
            signer_handle,
            out_document_id,
            out_document_json,
        )
    })
}

/// Create + broadcast an encrypted `txMetadata` document, letting RUST allocate
/// the per-document `encryptionKeyIndex` from authoritative Platform state
///. ABI-additive sibling of
/// [`platform_wallet_create_encrypted_document_with_signer`] — IDENTICAL
/// parameters minus `encryption_key_index`.
///
/// The host omits the index; the SDK counts the identity's existing txMetadata
/// documents on Platform and uses `1 + count` (dash-wallet's retired
/// `1 + countAllRequests()` semantics), serialized under the wallet's allocator
/// mutex so concurrent creates through the same process never collide.
/// Best-effort unique per device; a cross-device duplicate index is NOT
/// data-loss (see `IdentityWallet::allocate_encryption_key_index`). Every other
/// behavior (identity-key selection, AES derivation, sealing, master wiping,
/// broadcast) matches the explicit-index export.
///
/// # Safety
/// Same contract as [`platform_wallet_create_encrypted_document_with_signer`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_create_encrypted_document_with_signer_auto_index(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    version: u8,
    payload: *const u8,
    payload_len: usize,
    signer_handle: *mut SignerHandle,
    out_document_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // Same out-pointer contract as the explicit-index export: the address is
    // validated and its null sentinel published in the `extern "C"` frame,
    // before any other fallible input.
    check_ptr!(out_document_json);
    *out_document_json = ptr::null_mut();

    contain_panics(|| {
        create_encrypted_document_inner(
            wallet_handle,
            mnemonic_resolver_handle,
            owner_identity_id,
            contract_id,
            document_type_name,
            version,
            PayloadSource::Borrowed {
                ptr: payload,
                len: payload_len,
                index: None,
                borrow: PhantomData,
            },
            signer_handle,
            out_document_id,
            out_document_json,
        )
    })
}

/// Create + broadcast an encrypted `txMetadata` document while deferring the
/// caller's native plaintext copy until Rust has settled the index and key.
///
/// A Rust-ABI helper, not a C symbol: the JNI layer links this crate as an rlib
/// and calls it directly, so this adds no export to the C header and no second
/// implementation of the create — it converges on the same orchestration the
/// two C exports run.
///
/// It exists because a JVM `byte[]` cannot be pinned across the automatic index
/// query. JNI supplies only its declared length and a synchronous callback.
/// Rust validates the request, settles the explicit or automatic index,
/// prepares the complete encryption context, and only then invokes
/// `materialize_payload` exactly once. The returned
/// `Zeroizing<Vec<u8>>` is consumed by the shared create path and scrubbed as
/// soon as the encrypted properties are sealed, before broadcast begins.
///
/// Keeping that sequence in one Rust operation makes JNI a marshaling layer: it
/// never owns a native plaintext copy while a network allocation query or host
/// key lookup is in flight.
///
/// # Safety
/// Same pointer contract as
/// [`platform_wallet_create_encrypted_document_with_signer`], minus the payload:
/// `materialize_payload` must return exactly `payload_len` bytes. The helper
/// rejects a mismatch and drops both the returned zeroizing allocation and the
/// prepared zeroizing AES context without broadcasting. `out_document_json`
/// must point to writable storage for one `char *`; it is nulled before any
/// other fallible work and, on success, receives a string the caller MUST release with
/// `platform_wallet_string_free` (the ordinary free — this output is canonical
/// document JSON, ciphertext and metadata, no plaintext).
#[allow(clippy::too_many_arguments)]
pub unsafe fn create_encrypted_document_with_deferred_payload<'a>(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    encryption_key_index: Option<u32>,
    version: u8,
    payload_len: usize,
    materialize_payload: impl FnOnce() -> Result<Zeroizing<Vec<u8>>, PlatformWalletFFIResult> + 'a,
    signer_handle: *mut SignerHandle,
    out_document_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // Same out-pointer contract as the C exports: the address is validated and
    // its null sentinel published before any other fallible input, so a caller
    // following the documented contract never frees a pointer this call did not
    // own.
    check_ptr!(out_document_json);
    *out_document_json = ptr::null_mut();

    // Contained for the same reason as at the C exports, even though the
    // immediate caller is Rust. The deferred callback is owned by this closure,
    // so it is dropped without being called on any earlier rejection and any
    // materialized zeroizing payload is released during an unwind.
    contain_panics(move || {
        create_encrypted_document_inner(
            wallet_handle,
            mnemonic_resolver_handle,
            owner_identity_id,
            contract_id,
            document_type_name,
            version,
            PayloadSource::Deferred {
                len: payload_len,
                index: encryption_key_index,
                materialize: Box::new(materialize_payload),
            },
            signer_handle,
            out_document_id,
            out_document_json,
        )
    })
}

/// Rust-ABI body shared by every encrypted-document create entry point: the
/// explicit-index C export
/// ([`platform_wallet_create_encrypted_document_with_signer`]), the
/// SDK-allocated C export
/// ([`platform_wallet_create_encrypted_document_with_signer_auto_index`]), and
/// the deferred-payload Rust helper
/// ([`create_encrypted_document_with_deferred_payload`]).
///
/// Split out so a panic raised in here is caught before the `extern "C"` frame
/// (see [`contain_panics`]). Every caller has already validated
/// `out_document_json` and published its null sentinel.
///
/// The three differ only in where the plaintext comes from and how the index was
/// settled, which [`PayloadSource`] carries. When it reports no settled index
/// the per-document `encryptionKeyIndex` is allocated from Platform state before
/// the caller's plaintext is copied and before any key material is resolved, so
/// the allocation never crosses the broadcast await with a master in scope and
/// an oversized payload fails without reserving an index. Whichever route was
/// taken, the owned plaintext is released before the broadcast begins.
///
/// # Safety
/// Same contract as the `extern "C"` wrappers: every non-null pointer argument
/// must be valid for the duration of the call, a borrowed payload's pointer may
/// be null only when its length is `0`, and `out_document_json` must already
/// point to writable storage.
#[allow(clippy::too_many_arguments)]
unsafe fn create_encrypted_document_inner(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    version: u8,
    payload: PayloadSource<'_>,
    signer_handle: *mut SignerHandle,
    out_document_id: *mut u8,
    out_document_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    take_forced_inner_panic();

    check_ptr!(document_type_name);
    check_ptr!(out_document_id);

    // Everything decidable from the arguments alone — signer presence, payload
    // size, wire version, and a settled index's derivability — is checked
    // before the resolver, the network, the allocator, or any copy of the
    // caller's plaintext. A borrowed payload reports its DECLARED length, so an
    // over-large request is refused without its pointer ever being read; an
    // unsettled index is derivable by construction.
    let payload_len = payload.len();
    let preflight = tx_metadata_create_preflight_result(
        payload_len,
        version,
        payload.settled_index(),
        !signer_handle.is_null(),
    );
    if preflight.code != PlatformWalletFFIResultCode::Success {
        return preflight;
    }

    // A borrowed payload's ADDRESS is validated here, before anything is
    // allocated: a non-null pointer is required for a non-empty payload, and
    // null is valid only for a zero-length one. A deferred payload has no
    // address to check until its host callback returns owned bytes.
    if let PayloadSource::Borrowed { ptr, len, .. } = &payload {
        if *len != 0 && ptr.is_null() {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorNullPointer,
                "payload ptr is null",
            );
        }
    }

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));
    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();

    let signer_addr = signer_handle as usize;
    let owner_id_for_async = owner_id;
    let contract_id_for_async = contract_id_value;

    // Resolve the handle before any network wait or plaintext materialization.
    // The stored value is an `Arc`, so the process-wide storage guard is gone
    // before either stage begins and an invalid handle never causes a host copy.
    let Some(wallet_arc) = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, std::sync::Arc::clone)
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "requested wallet handle not found",
        );
    };
    let identity_wallet = wallet_arc.identity().clone();
    let identity_wallet_for_broadcast = identity_wallet.clone();

    // Settle the per-document encryptionKeyIndex and prepare the complete
    // encryption context before obtaining the owned plaintext.
    //
    // A borrowed pointer and a deferred host materializer converge here. The
    // host either supplied the index, or the SDK allocates it from Platform.
    // Allocating is a network round trip that needs only the payload's LENGTH,
    // so it is sequenced strictly ahead of key resolution and the copy: no
    // SDK-owned key master or plaintext copy is introduced during the network
    // wait. Key resolution follows and may synchronously call a host resolver;
    // managed-owner lookup, key selection, and AES derivation also finish before
    // the native plaintext allocation is created.
    let (declared_len, index, materialize): (usize, Option<u32>, DeferredPayloadMaterializer<'_>) =
        match payload {
            PayloadSource::Borrowed {
                ptr, len, index, ..
            } => (
                len,
                index,
                Box::new(move || {
                    // The pointer was validated above and is dereferenced only once
                    // the index is settled. The owned copy is scrubbed on drop.
                    Ok(Zeroizing::new(if len == 0 {
                        Vec::new()
                    } else {
                        slice::from_raw_parts(ptr, len).to_vec()
                    }))
                }),
            ),
            PayloadSource::Deferred {
                len,
                index,
                materialize,
            } => (len, index, materialize),
        };

    let identity_wallet_for_alloc = identity_wallet.clone();
    let sequenced = settle_index_prepare_encryption_and_materialize_payload(
        declared_len,
        || match index {
            Some(supplied) => Ok(supplied),
            None => {
                let document_type_for_alloc = document_type_str.clone();
                match try_block_on_worker(async move {
                    identity_wallet_for_alloc
                        .allocate_encryption_key_index(
                            &owner_id_for_async,
                            &contract_id_for_async,
                            &document_type_for_alloc,
                            declared_len,
                        )
                        .await
                }) {
                    Ok(allocated) => allocated.map_err(PlatformWalletFFIResult::from),
                    Err(failure) => Err(worker_failure_result(failure)),
                }
            }
        },
        |resolved_index| {
            let master_opt =
                tx_metadata_key_master_for_wallet(&wallet_arc, mnemonic_resolver_handle)
                    .map(|master| master.map(WipingMaster))?;
            let prepared = {
                let key_source = match master_opt.as_ref() {
                    Some(master) => TxMetadataKeySource::Master(&master.0),
                    None => TxMetadataKeySource::ResidentWallet,
                };
                identity_wallet.prepare_txmetadata_encryption_blocking(
                    &owner_id_for_async,
                    resolved_index,
                    version,
                    declared_len,
                    key_source,
                )
            };
            // The prepared context owns only the derived AES key. Erase the
            // much more powerful master before the host payload is copied.
            drop(master_opt);
            prepared.map_err(PlatformWalletFFIResult::from)
        },
        materialize,
    );
    let (_resolved_index, prepared, payload_vec) = match sequenced {
        Ok(sequenced) => sequenced,
        Err(failure) => return failure,
    };

    // Seal the wire blob SYNCHRONOUSLY; release the plaintext and prepared AES
    // context; only then broadcast. Neither the plaintext — whether this call
    // copied it or the host handed it over — nor any key material crosses the
    // broadcast await; only the sealed properties (ciphertext) do.
    let broadcast_outcome = seal_and_release_before_broadcasting(
        payload_vec,
        prepared,
        |plaintext, prepared| {
            prepared
                .seal(plaintext)
                .map_err(PlatformWalletFFIResult::from)
        },
        |properties_json| {
            // Fallible worker entry: a runtime that cannot be built, or a
            // worker that does not complete, becomes a value this export maps
            // instead of a panic that would reach the C frame.
            try_block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                // Generic create path (no key material in scope): fetches the
                // contract, sanitizes the hex `encryptedMetadata` into `Bytes`,
                // auto-selects the AUTHENTICATION signing key, and broadcasts on
                // the 8 MB worker stack.
                let confirmed: Document = identity_wallet_for_broadcast
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
        },
    );
    let result: Result<(Identifier, String), PlatformWalletError> = match broadcast_outcome {
        Ok(Ok(result)) => result,
        Ok(Err(failure)) => return worker_failure_result(failure),
        Err(failure) => return failure,
    };
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
/// The wire-compatible read counterpart of the legacy
/// `getTxMetaData(since, key)`, run in three deliberate stages. The staging is
/// a security guarantee, not an implementation detail:
///
/// 1. `IdentityWallet::fetch_raw_encrypted_documents` on a worker thread —
///    contract resolution and the paginated scan, with NO key material in
///    scope. A scan that fails or returns nothing ends here.
/// 2. Only if that scan produced candidates, the AES key source is acquired on
///    the ORIGINAL calling thread, so a host resolver callback runs on the
///    thread that entered this export rather than a runtime worker.
/// 3. `IdentityWallet::decrypt_fetched_documents_blocking` — synchronous derive and
///    decrypt, after which the resolved master is erased immediately.
///
/// Nothing secret is therefore alive across the contract fetch or the paginated
/// walk, both of which are unbounded waits (the SDK sets no request timeout),
/// and a fetch with nothing to decrypt never consults the host at all — which
/// matters where that consultation prompts the user.
///
/// The key source is selected by the wallet's capability: a key-resident wallet
/// derives in-process; an external-signable / watch-only wallet (the Android
/// and iOS apps) derives through `mnemonic_resolver_handle` — required non-null
/// for that shape, ignored otherwise (see `tx_metadata_key_master_for_wallet`).
///
/// Documents that cannot be derived or decrypted, and documents carrying an
/// unsupported wire version, are skipped and never abort the fetch.
///
/// A returned `payload` is NOT authenticated. The envelope is AES-256-CBC with
/// PKCS7 and no integrity tag, so a wrong key or modified ciphertext usually
/// fails the unpad and is skipped — but PKCS7 accepts a wrong plaintext often
/// enough that an element can carry opaque garbage. The caller must strictly
/// parse each `payload` (CBOR for `version` 0, protobuf for 1) and discard
/// anything that does not parse, rather than trusting its presence here.
///
/// On success `*out_documents_json` receives an owned NUL-terminated JSON array
/// containing decrypted, plaintext-equivalent data (release with
/// `platform_wallet_sensitive_string_free`; left null on any error). Treat the
/// allocation as read-only and pass its original, unmodified pointer to that
/// release function. Each element is
/// `{ "id": base58, "ownerId": base58, "keyIndex": u32, "encryptionKeyIndex":
/// u32, "version": u8, "updatedAt": u64|null, "payload": base64 }`, where
/// `payload` is the decrypted, opaque plaintext the caller parses (a protobuf
/// `TxMetadataBatch` for `version == 1`). Documents whose blob is malformed,
/// wrong-keyed, or carries an unsupported wire version are skipped rather than
/// failing the whole fetch.
///
/// # Safety
/// Every pointer below must stay valid for the whole synchronous duration of
/// this call; the call borrows them and retains none of them afterwards.
///
/// - `owner_identity_id` and `contract_id` must each point to 32 readable bytes.
/// - `document_type_name` must be a valid NUL-terminated C string of UTF-8.
/// - `mnemonic_resolver_handle` may be null for a wallet with resident private
///   keys, and must be live and non-null for an external-signable wallet.
///   There is no signer on this path: a fetch broadcasts nothing.
/// - `out_documents_json` must point to writable storage for one `char *`. It is
///   set to null before any other fallible work, so on EVERY error path the
///   caller is left holding null and must free nothing. On success it receives
///   ownership of a NUL-terminated C string.
///
/// That output carries DECRYPTED plaintext and MUST be released with
/// `platform_wallet_sensitive_string_free`, which wipes the allocation through
/// its terminating NUL. Passing it to the ordinary `platform_wallet_string_free`
/// would free the plaintext without scrubbing it. Pass the original, unmodified
/// pointer — the release function computes the length from it — and treat the
/// allocation as read-only until then.
///
/// The returned `PlatformWalletFFIResult` owns its message and must be released
/// with `platform_wallet_ffi_result_free`.
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
    // The sensitive out-parameter's ADDRESS is validated and its null sentinel
    // published before ANY other fallible input, so every later rejection —
    // including a bad document type or identifier — leaves the caller holding
    // null. This output carries decrypted plaintext and is released with
    // `platform_wallet_sensitive_string_free`, so a caller following the
    // documented contract must never be handed a stale pointer to free. This
    // runs in the `extern "C"` frame itself, so the sentinel is published even
    // if the inner function later fails in any way.
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

/// Rust-ABI inner for [`platform_wallet_fetch_encrypted_documents`], so a panic
/// in the decrypt path cannot reach the non-unwinding C frame.
///
/// The caller has already validated `out_documents_json`'s address and published
/// its null sentinel, so every return from here leaves the caller holding null
/// unless the sensitive JSON was successfully written.
///
/// # Safety
/// Same contract as the export.
unsafe fn fetch_encrypted_documents_inner(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    owner_identity_id: *const u8,
    contract_id: *const u8,
    document_type_name: *const c_char,
    since_ms: u64,
    out_documents_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(document_type_name);

    let owner_id = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let contract_id_value = unwrap_result_or_return!(read_identifier(contract_id));
    let document_type_str =
        unwrap_result_or_return!(CStr::from_ptr(document_type_name).to_str()).to_string();

    let owner_id_for_async = owner_id;
    let contract_id_for_async = contract_id_value;

    // Take an owned handle out of the shared storage and let the read guard go,
    // for the same reason as the create path: that guard is shared by every
    // wallet handle in the process, and the resolver call plus the paginated
    // fetch below are unbounded waits.
    let Some(wallet_arc) = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, std::sync::Arc::clone)
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "requested wallet handle not found",
        );
    };
    let identity_wallet = wallet_arc.identity().clone();

    // Phase 1 — NETWORK ONLY, on a worker. No key material exists yet: no host
    // resolver has been consulted and no master is in scope, so a scan that
    // fails or finds nothing costs the caller no prompt and leaves no secret
    // alive across the contract fetch or the paginated walk (both unbounded —
    // the SDK sets no request timeout).
    let raw_for_async = identity_wallet.clone();
    let document_type_for_async = document_type_str.clone();
    let raw_result: Result<Vec<(dpp::prelude::Identifier, Option<Document>)>, PlatformWalletError> =
        match try_block_on_worker(async move {
            raw_for_async
                .fetch_raw_encrypted_documents(
                    &owner_id_for_async,
                    &contract_id_for_async,
                    &document_type_for_async,
                    since_ms,
                )
                .await
        }) {
            Ok(result) => result,
            Err(failure) => return worker_failure_result(failure),
        };
    // Carried through unchanged, including entries the SDK could not
    // materialize: the decrypt stage records each skip, so an all-unmaterialized
    // page stays distinguishable from a page that was genuinely empty.
    let raw_docs = unwrap_result_or_return!(raw_result);

    // Nothing to decrypt: return the empty array without ever touching a key.
    if raw_docs.is_empty() {
        let sensitive_json = unwrap_result_or_return!(serialize_decrypted_documents(&[]));
        *out_documents_json = sensitive_json.into_raw();
        return PlatformWalletFFIResult::ok();
    }

    // Phase 2 — key acquisition, on the ORIGINAL calling thread. The host
    // mnemonic resolver is a caller-supplied callback; invoking it from the
    // thread that entered this export keeps it on the thread the host's own
    // contract was written for, rather than a Tokio worker.
    let master_opt = match tx_metadata_key_master_for_wallet(&wallet_arc, mnemonic_resolver_handle)
    {
        Ok(master) => master.map(WipingMaster),
        Err(failure) => return failure,
    };

    // Phase 3 — SYNCHRONOUS derive + decrypt, then wipe. No await separates the
    // acquisition above from the drop below, so the master is never live across
    // a network round trip. The guard scrubs on ordinary return, on an error
    // return and on an unwinding panic; an abort runs no destructor and is not
    // covered, and the write cannot reach a register copy the optimizer made.
    let key_source = match master_opt.as_ref() {
        Some(master) => TxMetadataKeySource::Master(&master.0),
        None => TxMetadataKeySource::ResidentWallet,
    };
    let decrypted =
        identity_wallet.decrypt_fetched_documents_blocking(&owner_id, &raw_docs, key_source);
    drop(master_opt);
    let docs = unwrap_result_or_return!(decrypted);

    let sensitive_json = unwrap_result_or_return!(serialize_decrypted_documents(&docs));
    *out_documents_json = sensitive_json.into_raw();
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

    // ── tx_metadata_key_master_for_wallet dispatch ──
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

    // ── Boundary contracts of the encrypted exports ─────────────────────────
    //
    // Every case below uses a wallet handle guaranteed absent from the storage
    // map, so the export's lookup misses (`NotFound`) and no resolver callback,
    // key derivation, allocator or broadcast ever runs. That miss is what makes
    // ordering observable: whichever check reports first is the check that ran
    // first. No invalid pointer is dereferenced — arguments that must be
    // non-null point at real test-owned storage the export only null-checks.

    /// A wallet handle guaranteed absent from the storage map.
    const UNKNOWN_WALLET_HANDLE: Handle = u64::MAX;

    /// A non-null pointer to real, test-owned storage, used where the export
    /// only checks for null and never dereferences.
    fn opaque_non_null<T>(storage: &mut u8) -> *mut T {
        storage as *mut u8 as *mut T
    }

    fn platform_wallet_ffi_max_plaintext_len() -> usize {
        platform_wallet::wallet::identity::crypto::tx_metadata::MAX_TX_METADATA_PLAINTEXT_LEN
    }

    /// The shared argument gate pins both sides of the index ceiling, the
    /// version set, and the signer precondition.
    #[test]
    fn the_shared_argument_gate_pins_both_sides_of_the_index_ceiling() {
        use platform_wallet::wallet::identity::crypto::tx_metadata::MAX_TX_METADATA_ENCRYPTION_KEY_INDEX;

        assert_eq!(
            tx_metadata_create_preflight_result(
                8,
                1,
                Some(MAX_TX_METADATA_ENCRYPTION_KEY_INDEX),
                true
            )
            .code,
            PlatformWalletFFIResultCode::Success,
            "the maximum derivable index is a valid argument and must pass"
        );
        assert_eq!(
            tx_metadata_create_preflight_result(
                8,
                1,
                Some(MAX_TX_METADATA_ENCRYPTION_KEY_INDEX + 1),
                true
            )
            .code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "one past the maximum has no derivable key and must be refused"
        );
        assert_eq!(
            tx_metadata_create_preflight_result(8, 1, None, true).code,
            PlatformWalletFFIResultCode::Success,
            "an index about to be allocated is derivable by construction"
        );
        assert_eq!(
            tx_metadata_create_preflight_result(8, 2, Some(1), true).code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "a version the legacy stack cannot decode must be refused"
        );
        assert_eq!(
            tx_metadata_create_preflight_result(
                platform_wallet_ffi_max_plaintext_len() + 1,
                1,
                Some(1),
                true
            )
            .code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "a payload that cannot be sealed must be refused"
        );
        assert_eq!(
            tx_metadata_create_preflight_result(8, 1, Some(1), false).code,
            PlatformWalletFFIResultCode::ErrorNullPointer,
            "a create with no signer cannot broadcast, so the gate must refuse it \
             alongside the wallet-protocol arguments"
        );
        assert_eq!(
            tx_metadata_create_preflight_result(
                platform_wallet_ffi_max_plaintext_len(),
                1,
                Some(1),
                true
            )
            .code,
            PlatformWalletFFIResultCode::Success,
            "the largest sealable payload is a valid argument"
        );
    }

    /// A failed index resolution resolves no key and copies no plaintext.
    #[test]
    fn a_failed_index_resolution_never_copies_the_plaintext() {
        let key_resolved = std::cell::Cell::new(false);
        let copied = std::cell::Cell::new(false);

        let sequenced = settle_index_prepare_encryption_and_materialize_payload(
            3,
            || {
                Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorUnknown,
                    "allocation failed",
                ))
            },
            |_| {
                key_resolved.set(true);
                Ok(())
            },
            || {
                copied.set(true);
                Ok(Zeroizing::new(vec![1, 2, 3]))
            },
        );

        assert!(sequenced.is_err(), "the resolution failure must propagate");
        assert!(!key_resolved.get());
        assert!(
            !copied.get(),
            "a request that cannot proceed must not copy the caller's plaintext"
        );
    }

    /// Complete encryption preparation must finish before the native plaintext
    /// copy is created, because context resolution and derivation can fail or
    /// block independently of the payload.
    #[test]
    fn encryption_is_prepared_before_the_plaintext_is_materialized() {
        let order = std::cell::RefCell::new(Vec::new());

        let (index, secret, payload) = settle_index_prepare_encryption_and_materialize_payload(
            3,
            || {
                order.borrow_mut().push("resolve-index");
                Ok(7)
            },
            |resolved_index| {
                order.borrow_mut().push("prepare-encryption");
                assert_eq!(resolved_index, 7);
                Ok(11)
            },
            || {
                order.borrow_mut().push("materialize");
                Ok(Zeroizing::new(vec![1, 2, 3]))
            },
        )
        .expect("all stages succeed");

        assert_eq!(index, 7);
        assert_eq!(secret, 11);
        assert_eq!(payload.as_slice(), [1, 2, 3]);
        assert_eq!(
            order.into_inner(),
            vec!["resolve-index", "prepare-encryption", "materialize"]
        );
    }

    /// An encryption-preparation failure must return while the deferred host
    /// materializer is still untouched, so no native plaintext copy is created
    /// for a request that cannot be encrypted.
    #[test]
    fn failed_encryption_preparation_never_materializes_plaintext() {
        let materialize_calls = std::cell::Cell::new(0);

        let outcome = settle_index_prepare_encryption_and_materialize_payload(
            3,
            || Ok(7),
            |_| {
                Err::<u8, _>(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorUnknown,
                    "encryption preparation failed",
                ))
            },
            || {
                materialize_calls.set(materialize_calls.get() + 1);
                Ok(Zeroizing::new(vec![1, 2, 3]))
            },
        );

        assert!(outcome.is_err());
        assert_eq!(materialize_calls.get(), 0);
    }

    /// A failed allocation must leave the deferred materializer untouched.
    #[test]
    fn deferred_payload_is_not_materialized_when_index_resolution_fails() {
        let materialize_calls = std::cell::Cell::new(0);

        let outcome = settle_index_prepare_encryption_and_materialize_payload(
            3,
            || {
                Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorUnknown,
                    "allocation failed",
                ))
            },
            |_| Ok(()),
            || {
                materialize_calls.set(materialize_calls.get() + 1);
                Ok(Zeroizing::new(vec![1, 2, 3]))
            },
        );

        assert!(outcome.is_err());
        assert_eq!(materialize_calls.get(), 0);
    }

    /// The declared length is part of the deferred-materialization contract.
    /// A mismatched buffer drops the resolved key and is rejected before broadcast.
    #[test]
    fn deferred_payload_rejects_a_materialized_length_mismatch() {
        let materialize_calls = std::cell::Cell::new(0);

        let outcome = settle_index_prepare_encryption_and_materialize_payload(
            3,
            || Ok(7),
            |_| Ok(()),
            || {
                materialize_calls.set(materialize_calls.get() + 1);
                Ok(Zeroizing::new(vec![1, 2]))
            },
        );

        assert_eq!(
            outcome
                .expect_err("the materialized length must match")
                .code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(materialize_calls.get(), 1);
    }

    // ── The owned plaintext dies before the broadcast begins ────────────────
    //
    // A host that cannot pin its own buffer across the call (the JVM bridge)
    // hands its ONLY native plaintext copy over by value. What makes that
    // transfer worth anything is what happens to the copy next: it must be
    // sealed, released, and only THEN broadcast. The broadcast is an unbounded
    // network wait — the SDK sets no request timeout — so a copy still alive
    // when it starts is a copy alive for however long the network takes.
    //
    // Recorded through the same seam production runs, so the order asserted
    // here is the order the exports run.

    /// The owned plaintext copy, standing in for `Zeroizing<Vec<u8>>` and
    /// recording the moment its storage is released.
    struct ReleaseRecorder<'a> {
        events: &'a std::cell::RefCell<Vec<&'static str>>,
        label: &'static str,
    }

    impl Drop for ReleaseRecorder<'_> {
        fn drop(&mut self) {
            self.events.borrow_mut().push(self.label);
        }
    }

    /// The plaintext and the resolved key material are both gone before the
    /// broadcast starts.
    #[test]
    fn the_owned_plaintext_is_released_before_the_broadcast_begins() {
        let events = std::cell::RefCell::new(Vec::new());

        let outcome = seal_and_release_before_broadcasting(
            ReleaseRecorder {
                events: &events,
                label: "release-plaintext",
            },
            ReleaseRecorder {
                events: &events,
                label: "release-secret",
            },
            |_plaintext, _secret| {
                events.borrow_mut().push("seal");
                Ok::<_, PlatformWalletFFIResult>("ciphertext")
            },
            |sealed| {
                events.borrow_mut().push("broadcast");
                sealed
            },
        );

        assert_eq!(
            outcome.expect("both stages succeed in this case"),
            "ciphertext"
        );
        assert_eq!(
            events.into_inner(),
            vec!["seal", "release-plaintext", "release-secret", "broadcast"],
            "the plaintext and the resolved key material must both be released \
             BEFORE the broadcast begins; releasing them after it returns keeps \
             them resident for the whole of an unbounded network wait"
        );
    }

    /// A seal that fails releases both secrets and broadcasts nothing.
    #[test]
    fn a_failed_seal_releases_the_plaintext_and_never_broadcasts() {
        let events = std::cell::RefCell::new(Vec::new());

        let outcome = seal_and_release_before_broadcasting(
            ReleaseRecorder {
                events: &events,
                label: "release-plaintext",
            },
            ReleaseRecorder {
                events: &events,
                label: "release-secret",
            },
            |_plaintext, _secret| {
                Err::<&str, _>(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    "derivation failed",
                ))
            },
            |sealed| {
                events.borrow_mut().push("broadcast");
                sealed
            },
        );

        assert!(outcome.is_err(), "the seal failure must propagate");
        assert_eq!(
            events.into_inner(),
            vec!["release-plaintext", "release-secret"],
            "a create that cannot seal must still release what it holds, and must \
             not reach the network at all"
        );
    }

    /// A null payload with a non-zero length is rejected from the arguments
    /// alone — before an index is consumed and before the network is touched.
    #[test]
    fn create_encrypted_auto_index_rejects_a_null_payload_before_allocating() {
        let mut out_json: *mut c_char = ptr::null_mut();
        let mut out_id = [0u8; 32];
        let doc_type = CString::new("txMetadata").expect("no interior NUL");
        let owner = [1u8; 32];
        let contract = [2u8; 32];
        let mut signer_storage = 0u8;

        let result = unsafe {
            platform_wallet_create_encrypted_document_with_signer_auto_index(
                UNKNOWN_WALLET_HANDLE,
                ptr::null_mut(),
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                1,
                ptr::null(),
                8,
                opaque_non_null(&mut signer_storage),
                out_id.as_mut_ptr(),
                &mut out_json,
            )
        };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorNullPointer,
            "a null payload with a non-zero length must be rejected from the \
             arguments, not after an index has been allocated"
        );
        assert!(out_json.is_null());
    }

    /// The create export publishes its documented null sentinel before any
    /// other fallible validation, so a caller following the contract never frees
    /// a pointer this call did not own.
    #[test]
    fn create_encrypted_publishes_null_json_out_before_other_validation() {
        let mut sentinel_storage: c_char = 0x7f;
        let mut out_json: *mut c_char = &mut sentinel_storage;
        let mut out_id = [0u8; 32];
        let owner = [1u8; 32];
        let contract = [2u8; 32];
        let mut signer_storage = 0u8;

        // A NULL document type trips a check that runs after the sentinel is
        // published, so the sentinel must already have been cleared.
        let result = unsafe {
            platform_wallet_create_encrypted_document_with_signer(
                UNKNOWN_WALLET_HANDLE,
                ptr::null_mut(),
                owner.as_ptr(),
                contract.as_ptr(),
                ptr::null(),
                1,
                1,
                ptr::null(),
                0,
                opaque_non_null(&mut signer_storage),
                out_id.as_mut_ptr(),
                &mut out_json,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert!(
            out_json.is_null(),
            "the out pointer must be nulled before any other fallible input is \
             validated, not only on the success path"
        );
    }

    /// Same contract on the auto-index export.
    #[test]
    fn create_encrypted_auto_index_publishes_null_json_out_before_other_validation() {
        let mut sentinel_storage: c_char = 0x7f;
        let mut out_json: *mut c_char = &mut sentinel_storage;
        let mut out_id = [0u8; 32];
        let owner = [1u8; 32];
        let contract = [2u8; 32];
        let mut signer_storage = 0u8;

        let result = unsafe {
            platform_wallet_create_encrypted_document_with_signer_auto_index(
                UNKNOWN_WALLET_HANDLE,
                ptr::null_mut(),
                owner.as_ptr(),
                contract.as_ptr(),
                ptr::null(),
                1,
                ptr::null(),
                0,
                opaque_non_null(&mut signer_storage),
                out_id.as_mut_ptr(),
                &mut out_json,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert!(out_json.is_null());
    }

    /// The fetch export's output carries decrypted plaintext and is released
    /// with the sensitive free, so its sentinel must be published before every
    /// other fallible input too.
    #[test]
    fn fetch_encrypted_publishes_null_json_out_before_other_validation() {
        let mut sentinel_storage: c_char = 0x7f;
        let mut out_json: *mut c_char = &mut sentinel_storage;
        let owner = [1u8; 32];
        let contract = [2u8; 32];

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
            "a stale non-null pointer here would be freed with the sensitive free \
             by a caller following the documented contract"
        );
    }

    /// An oversized length is rejected without the payload pointer ever being
    /// read, so a caller that passes a length larger than its buffer is refused
    /// rather than over-read.
    #[test]
    fn create_encrypted_rejects_oversized_length_before_touching_the_payload_pointer() {
        let mut out_json: *mut c_char = ptr::null_mut();
        let mut out_id = [0u8; 32];
        let doc_type = CString::new("txMetadata").expect("no interior NUL");
        let owner = [1u8; 32];
        let contract = [2u8; 32];
        let mut signer_storage = 0u8;
        // One real byte, with a declared length far beyond it. The size gate
        // rejects from the length alone, so this is never dereferenced.
        let one_byte = [0u8; 1];

        let result = unsafe {
            platform_wallet_create_encrypted_document_with_signer_auto_index(
                UNKNOWN_WALLET_HANDLE,
                ptr::null_mut(),
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                1,
                one_byte.as_ptr(),
                platform_wallet_ffi_max_plaintext_len() + 1,
                opaque_non_null(&mut signer_storage),
                out_id.as_mut_ptr(),
                &mut out_json,
            )
        };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "the declared length alone must decide this, before any read"
        );
        assert!(out_json.is_null());
    }

    // ── Runtime and worker failures are values, not panics ──────────────────

    /// A runtime that cannot be built surfaces as a mapped result rather than a
    /// panic crossing the C frame.
    #[test]
    fn a_runtime_init_failure_maps_to_a_result_instead_of_panicking() {
        crate::runtime::force_runtime_init_failure_once();
        let outcome = try_block_on_worker(async { 1u8 });

        let failure = outcome.expect_err("the forced failure must be reported");
        assert_eq!(failure, crate::runtime::WorkerFailure::RuntimeInit);
        assert_eq!(
            worker_failure_result(failure).code,
            PlatformWalletFFIResultCode::ErrorUnknown,
            "neither stage is the caller's fault, so both map to the unknown code"
        );

        // The forcing is one-shot: the shared runtime is untouched and the next
        // call still works.
        assert_eq!(
            try_block_on_worker(async { 2u8 }).expect("the next call must succeed"),
            2
        );
    }

    /// A worker that does not complete surfaces the same way.
    #[test]
    fn a_worker_join_failure_maps_to_a_result_instead_of_panicking() {
        crate::runtime::force_worker_join_failure_once();
        let outcome = try_block_on_worker(async { 1u8 });

        let failure = outcome.expect_err("the forced failure must be reported");
        assert_eq!(failure, crate::runtime::WorkerFailure::WorkerJoin);
        assert_eq!(
            worker_failure_result(failure).code,
            PlatformWalletFFIResultCode::ErrorUnknown
        );
        assert_eq!(
            try_block_on_worker(async { 3u8 }).expect("the next call must succeed"),
            3
        );
    }

    /// A panic inside an inner function is contained before the `extern "C"`
    /// frame, where unwinding into a non-unwinding frame would abort the host.
    ///
    /// Only meaningful where unwinding exists: under `panic = "abort"` the
    /// process is gone at the point of the panic and nothing can catch it.
    #[cfg(panic = "unwind")]
    #[test]
    fn an_inner_panic_is_contained_before_the_extern_c_boundary() {
        let result = contain_panics(|| {
            take_forced_inner_panic();
            PlatformWalletFFIResult::ok()
        });
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::Success,
            "with no panic forced, the inner result passes through unchanged"
        );

        force_inner_panic_once();
        let contained = contain_panics(|| {
            take_forced_inner_panic();
            PlatformWalletFFIResult::ok()
        });
        assert_eq!(
            contained.code,
            PlatformWalletFFIResultCode::ErrorUnknown,
            "a panic must become an ordinary error value rather than unwinding \
             into the C frame"
        );
    }

    // ── The host resolver is consulted only when there is something to decrypt ──
    //
    // A wallet registered through the manager is stored external-signable, so its
    // txMetadata key must come from the host mnemonic resolver — on a device that
    // callback can prompt the user. The fetch export must therefore run its
    // network scan FIRST and consult the resolver only if that scan produced
    // candidates. Counting the callback is what makes the ordering observable:
    // if acquisition ran before the scan, the count would be 1 in every case
    // below, including the ones that never had anything to decrypt.

    /// Host-side resolver context: the phrase to hand back, plus a count of how
    /// many times the host was consulted.
    struct ResolverContext {
        /// Derived at runtime from all-zero entropy so no recovery phrase is
        /// committed to the repository.
        phrase: String,
        calls: std::sync::atomic::AtomicUsize,
    }

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

    struct ResolverFixture {
        wallet_handle: Handle,
        resolver: *mut MnemonicResolverHandle,
        context: *mut ResolverContext,
        manager_handle: Handle,
        sdk: Box<dash_sdk::Sdk>,
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
                let _ = crate::wallet::platform_wallet_destroy(self.wallet_handle);
                let _ = crate::manager::platform_wallet_manager_destroy(self.manager_handle);
                rs_sdk_ffi::dash_sdk_mnemonic_resolver_destroy(self.resolver);
                drop(Box::from_raw(self.context));
            }
        }
    }

    /// An identity carrying the ECDSA key the txMetadata derivation selects.
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
    /// path (which stores it external-signable), give it a resident identity
    /// slot, and wire a counting host resolver.
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

        let mnemonic =
            Mnemonic::from_entropy(&[0u8; 16], Language::English).expect("16 bytes of entropy");
        let phrase = mnemonic.phrase().to_string();

        // Pin the protocol version so a registered query expectation encodes the
        // same way the production scan encodes its request.
        let sdk = Box::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_version(dpp::version::PlatformVersion::latest())
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
            release_fn: None,
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
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        let mnemonic_c = CString::new(phrase.clone()).expect("no interior NUL");
        let mut wallet_handle: Handle = 0;
        let mut wallet_id = [0u8; 32];
        let result = unsafe {
            crate::manager::platform_wallet_manager_create_wallet_from_mnemonic(
                manager_handle,
                mnemonic_c.as_ptr(),
                crate::FFINetwork::Testnet,
                0,
                &mut wallet_handle,
                &mut wallet_id,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

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
            sdk,
        }
    }

    /// A create whose owner is not managed by the wallet must fail before the
    /// deferred host payload is copied into native memory. The resolver is
    /// deliberately valid so the request reaches owner-context resolution; a
    /// null or failing resolver would make this pass without exercising the
    /// plaintext-lifetime bug.
    #[test]
    fn a_missing_owner_context_never_materializes_deferred_plaintext() {
        let fixture = resolver_fixture();
        let missing_owner = [0x44u8; 32];
        let contract = [0x55u8; 32];
        let doc_type = CString::new("txMetadata").expect("no interior NUL");
        let materialize_calls = std::cell::Cell::new(0);
        let mut signer_storage = 0u8;
        let mut out_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();

        let result = unsafe {
            create_encrypted_document_with_deferred_payload(
                fixture.wallet_handle,
                fixture.resolver,
                missing_owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                Some(1),
                1,
                3,
                || {
                    materialize_calls.set(materialize_calls.get() + 1);
                    Ok(Zeroizing::new(vec![1, 2, 3]))
                },
                opaque_non_null(&mut signer_storage),
                out_id.as_mut_ptr(),
                &mut out_json,
            )
        };

        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(
            !result.message.is_null(),
            "the missing-owner error must carry its typed message"
        );
        let message = unsafe { CStr::from_ptr(result.message) }
            .to_str()
            .expect("wallet errors are valid UTF-8");
        assert!(
            message.contains("Identity not found"),
            "the request must reach the missing-owner failure, not stop at the resolver: {message}"
        );
        assert_eq!(
            materialize_calls.get(),
            0,
            "a request with no encryption context must not create a native plaintext copy"
        );
        assert!(out_json.is_null());
    }

    /// Drive the real fetch export, returning the result and the JSON the export
    /// produced (`None` when it left the sensitive out-pointer null). The
    /// allocation is released through the sensitive free before returning.
    fn fetch_encrypted_with(
        fixture: &ResolverFixture,
    ) -> (PlatformWalletFFIResult, Option<String>) {
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
        let json = if out_json.is_null() {
            None
        } else {
            let rendered = unsafe { CStr::from_ptr(out_json) }
                .to_str()
                .expect("the serializer guarantees ASCII")
                .to_string();
            unsafe { crate::types::platform_wallet_sensitive_string_free(out_json) };
            Some(rendered)
        };
        (result, json)
    }

    /// A scan that FAILS must never have consulted the host resolver.
    ///
    /// No contract fetch is registered on the mock, so the very first network
    /// step fails. If key acquisition ran before the scan the count would be 1
    /// here, and a device user would have been prompted for a fetch that could
    /// never return anything.
    #[test]
    fn a_failing_fetch_never_consults_the_host_resolver() {
        let fixture = resolver_fixture();

        let (result, json) = fetch_encrypted_with(&fixture);

        assert_ne!(
            result.code,
            PlatformWalletFFIResultCode::Success,
            "the scan cannot succeed with no registered contract"
        );
        assert!(
            json.is_none(),
            "the sensitive out pointer stays null on error"
        );
        assert_eq!(
            fixture.resolver_calls(),
            0,
            "a failed scan must not have prompted the host for key material"
        );
    }

    /// A scan that returns NOTHING must never have consulted the host resolver.
    ///
    /// The contract resolves and the page comes back empty, so the export gets
    /// all the way through its network work and then has nothing to decrypt.
    #[test]
    fn an_empty_fetch_never_consults_the_host_resolver() {
        let mut fixture = resolver_fixture();

        let contract = std::sync::Arc::new(
            dpp::tests::fixtures::get_data_contract_fixture(None, 0, dpp::version::LATEST_VERSION)
                .data_contract_owned(),
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("registration runtime");
        runtime.block_on(async {
            fixture
                .sdk
                .mock()
                .expect_fetch(Identifier::from([4u8; 32]), Some((*contract).clone()))
                .await
                .expect("register the contract fetch");
            // The exact query the production loop issues, answered with a short
            // (empty) page so the scan completes rather than failing.
            let empty: dash_sdk::query_types::Documents = Default::default();
            fixture
                .sdk
                .mock()
                .expect_fetch_many(
                    empty_page_query(std::sync::Arc::clone(&contract)),
                    Some(empty),
                )
                .await
                .expect("register the empty page");
        });

        let (result, json) = fetch_encrypted_with(&fixture);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::Success,
            "a scan that completes with no documents is a success, not an error"
        );
        assert_eq!(
            json.as_deref(),
            Some("[]"),
            "the export must still publish an owned, empty JSON array"
        );
        assert_eq!(
            fixture.resolver_calls(),
            0,
            "a scan that produced no candidate documents must not have prompted \
             the host for key material"
        );
    }

    /// A non-empty scan consults the host resolver exactly once, and only after
    /// the scan itself has run.
    ///
    /// The page is sealed under the SAME seed the counting resolver hands back,
    /// so the export's own derivation opens it — which means the decrypt stage
    /// genuinely ran rather than being skipped. Together with the two cases
    /// above (which prove a failing or empty scan consults the host zero times)
    /// this pins the ordering: acquisition happens on the candidates-exist path
    /// and on no other.
    #[test]
    fn a_non_empty_fetch_consults_the_host_resolver_exactly_once_after_the_scan() {
        use platform_wallet::wallet::identity::crypto::tx_metadata::{
            derive_tx_metadata_key_from_master, seal_tx_metadata,
        };

        const ENCRYPTION_KEY_INDEX: u32 = 1;
        const PLAINTEXT: &[u8] = b"memo=ffi-round-trip";

        let mut fixture = resolver_fixture();
        let network = fixture.sdk.network;

        // Seal with the resolver's own seed, in a block so the sealing secrets
        // do not outlive it. The master zeroizes on drop; the explicit erase
        // additionally narrows the scalar's lifetime within this block.
        let blob = {
            use key_wallet::bip32::ExtendedPrivKey;
            use key_wallet::mnemonic::{Language, Mnemonic};

            let seed = zeroize::Zeroizing::new(
                Mnemonic::from_entropy(&[0u8; 16], Language::English)
                    .expect("16 bytes of entropy")
                    .to_seed(""),
            );
            let mut master = ExtendedPrivKey::new_master(network, seed.as_ref())
                .expect("master from the resolver's own seed");
            // Slot 0 and key id 2 are what `fixture_identity` registers, so this
            // is the derivation the export will re-run.
            let aes_key =
                derive_tx_metadata_key_from_master(&master, network, 0, 2, ENCRYPTION_KEY_INDEX)
                    .expect("derive");
            let iv = [0x6Du8; 16];
            let sealed = seal_tx_metadata(&aes_key, 1, &iv, PLAINTEXT).expect("seal");
            master.private_key.non_secure_erase();
            sealed
        };

        let contract = std::sync::Arc::new(
            dpp::tests::fixtures::get_data_contract_fixture(None, 0, dpp::version::LATEST_VERSION)
                .data_contract_owned(),
        );
        let doc_id = Identifier::from([0x77u8; 32]);
        let mut properties: BTreeMap<String, dpp::platform_value::Value> = Default::default();
        properties.insert("keyIndex".to_string(), dpp::platform_value::Value::U32(2));
        properties.insert(
            "encryptionKeyIndex".to_string(),
            dpp::platform_value::Value::U32(ENCRYPTION_KEY_INDEX),
        );
        properties.insert(
            "encryptedMetadata".to_string(),
            dpp::platform_value::Value::Bytes(blob),
        );
        let document = Document::V0(dpp::document::DocumentV0 {
            id: doc_id,
            owner_id: Identifier::from(FIXTURE_OWNER),
            properties,
            revision: Some(1),
            created_at: None,
            updated_at: Some(1_700_000_000_000),
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("registration runtime");
        runtime.block_on(async {
            fixture
                .sdk
                .mock()
                .expect_fetch(Identifier::from([4u8; 32]), Some((*contract).clone()))
                .await
                .expect("register the contract fetch");
            let mut page: dash_sdk::query_types::Documents = Default::default();
            page.insert(doc_id, Some(document));
            fixture
                .sdk
                .mock()
                .expect_fetch_many(
                    empty_page_query(std::sync::Arc::clone(&contract)),
                    Some(page),
                )
                .await
                .expect("register the single-document page");
        });

        assert_eq!(
            fixture.resolver_calls(),
            0,
            "nothing has consulted the host before the export is entered"
        );

        let (result, json) = fetch_encrypted_with(&fixture);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::Success,
            "the page was sealed under the resolver's own seed, so it must decrypt"
        );
        let json = json.expect("a successful fetch publishes an owned JSON array");
        // The registered query was consumed and its document decrypted: the
        // payload only appears if the decrypt stage ran on what the scan
        // returned.
        let expected_payload = base64_of(PLAINTEXT);
        assert!(
            json.contains(&expected_payload),
            "the decrypted payload must reach the caller; got {json}"
        );
        assert_eq!(
            fixture.resolver_calls(),
            1,
            "the host must be consulted exactly once, and only because the scan \
             produced a candidate — a second call would mean the key was acquired \
             per document rather than once for the batch"
        );
    }

    /// Standard base64 of `bytes`, matching the serializer's payload encoding.
    fn base64_of(bytes: &[u8]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    /// The exact `DocumentQuery` the production scan issues for its first page.
    fn empty_page_query(
        contract: std::sync::Arc<dpp::prelude::DataContract>,
    ) -> dash_sdk::platform::DocumentQuery {
        use dash_sdk::drive::query::{OrderClause, WhereClause, WhereOperator};
        use dpp::platform_value::platform_value;

        dash_sdk::platform::DocumentQuery {
            select: dash_sdk::drive::query::SelectProjection::documents(),
            data_contract: contract,
            document_type_name: "txMetadata".to_string(),
            where_clauses: vec![
                WhereClause {
                    field: "$ownerId".to_string(),
                    operator: WhereOperator::Equal,
                    value: platform_value!(Identifier::from(FIXTURE_OWNER)),
                },
                WhereClause {
                    field: "$updatedAt".to_string(),
                    operator: WhereOperator::GreaterThanOrEquals,
                    value: platform_value!(0u64),
                },
            ],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![OrderClause {
                field: "$updatedAt".to_string(),
                ascending: true,
            }],
            limit: 100,
            offset: None,
            start: None,
        }
    }

    /// The fetch path's output is built by the sensitive serializer and is
    /// released by the sensitive free — not by the ordinary string free.
    ///
    /// This is what keeps decrypted plaintext in an allocation that is wiped on
    /// release. Routing it back through an ordinary `CString` would leave the
    /// plaintext in a non-zeroizing allocation, so the ownership is asserted
    /// here rather than left to the export's call site alone.
    #[test]
    fn the_fetch_output_is_owned_and_released_by_the_sensitive_contract() {
        let serialized =
            serialize_decrypted_documents(&[]).expect("an empty document set serializes");
        let raw = serialized.into_raw();
        assert!(!raw.is_null(), "the serializer hands back an owned pointer");

        let rendered = unsafe { CStr::from_ptr(raw) }
            .to_str()
            .expect("the serializer guarantees ASCII");
        assert_eq!(
            rendered, "[]",
            "the wire shape is the same JSON array the ordinary path produced"
        );

        // Released through the sensitive free, which wipes the allocation
        // including its terminator. The ordinary free must never be used here.
        unsafe { crate::types::platform_wallet_sensitive_string_free(raw) };
    }
}
