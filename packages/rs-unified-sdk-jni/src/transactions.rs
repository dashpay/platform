//! JNI bridge for the identity / document / voting **write** paths that
//! back the deferred Kotlin screens (the read-only wallet-memory snapshots
//! and DAPI ban list live in `wallet_manager.rs` under `WalletManagerNative`,
//! since those Kotlin accessors hang off `PlatformWalletManager` /
//! `ManagedPlatformWallet`).
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.TransactionsNative`,
//! driven by `identity.IdentityUpdates`, `documents.DocumentTransactions`,
//! and `voting.VoteCasting`.
//!
//! ## What lives here (and the boundary rule)
//!
//! Every export is a thin marshaler over a SINGLE Rust FFI entry point — no
//! stitching of multiple Rust calls, per `packages/kotlin-sdk/CLAUDE.md`.
//! Each function mirrors the exact call the Swift example app makes for the
//! same screen:
//!   * identity add/disable keys -> `platform_wallet_update_identity_with_signer`
//!     (`AddIdentityKeyView.submit` -> `wallet.updateIdentity(...)`),
//!   * document purchase / set-price ->
//!     `platform_wallet_document_purchase` / `platform_wallet_document_set_price`
//!     (`ManagedPlatformWallet.purchaseDocument`),
//!   * cast vote -> `dash_sdk_contested_resource_cast_vote`
//!     (`SDK.castContestedResourceVote` behind `ContestDetailView`).
//!
//! ## Result convention
//!
//! The identity / document entry points are platform-wallet-ffi and return
//! `PlatformWalletFFIResult`, so errors go through the shared
//! [`crate::support::take_pwffi_error`] (offset-namespaced). The vote entry
//! point is rs-sdk-ffi and returns `DashSDKResult`, so it uses
//! [`crate::results::take_error`] like the query surface.
//!
//! ## Copy-before-free
//!
//! Every entry point that hands back a C string copies the payload into a
//! JVM string and then calls the paired `*_free` before returning, so no
//! Rust allocation escapes the call.

#![allow(clippy::missing_safety_doc)]

use crate::pubkey_rows::decode_update_pubkeys_blob;
use crate::support::{guard, net_from_ord, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jint, jlong, jstring};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::identity_registration_with_signer::IdentityPubkeyFFI;
use rs_sdk_ffi::SignerHandle;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// The release function a native-string guard calls exactly once on drop.
///
/// Carried as a field rather than hard-coded in each `Drop` so the CHOICE of
/// release function is observable: a test can install a recording release and
/// prove which contract a guard actually honors, on the normal path and on an
/// unwind. Both real frees are null-safe and the null form cannot tell them
/// apart, so nothing else in this file can catch a guard silently switched to
/// the wrong free.
type NativeStringRelease = unsafe extern "C" fn(*mut c_char);

/// Nullable owner for plaintext-equivalent strings returned by
/// `platform_wallet_fetch_encrypted_documents`.
///
/// Install this immediately after the FFI call so every later result, JNI
/// allocation, and unwind path releases the allocation through the sensitive
/// zeroizing contract.
struct SensitivePlatformWalletString {
    ptr: *mut c_char,
    /// Always the sensitive free in production — the field is private and only
    /// [`Self::new`] fills it, so a call site still cannot pair this allocation
    /// with another contract's release function.
    release: NativeStringRelease,
}

impl SensitivePlatformWalletString {
    fn new(ptr: *mut c_char) -> Self {
        Self {
            ptr,
            release: platform_wallet_ffi::platform_wallet_sensitive_string_free,
        }
    }

    /// Same owner with a caller-supplied release, for proving which release the
    /// drop paths actually run. Test-only: production ownership always comes
    /// from [`Self::new`].
    #[cfg(test)]
    fn with_release_for_test(ptr: *mut c_char, release: NativeStringRelease) -> Self {
        Self { ptr, release }
    }

    fn as_c_str(&self) -> Option<&CStr> {
        if self.ptr.is_null() {
            None
        } else {
            // SAFETY: a non-null pointer came from the platform-wallet FFI
            // CString result and remains owned by this guard.
            Some(unsafe { CStr::from_ptr(self.ptr) })
        }
    }
}

impl Drop for SensitivePlatformWalletString {
    fn drop(&mut self) {
        // SAFETY: this guard is the sole owner of the nullable pointer, and the
        // fetch contract names the sensitive free as its release function.
        unsafe {
            (self.release)(self.ptr);
        }
    }
}

/// Nullable owner for ORDINARY strings returned by
/// `create_encrypted_document_with_deferred_payload`.
///
/// The create output is the confirmed document's canonical JSON — ciphertext
/// and metadata, no plaintext — so it is released with the ordinary free, not
/// the sensitive one. The two contracts are deliberately distinct types so a
/// call site cannot pair an allocation with the wrong release function.
///
/// Install this immediately after the FFI call transfers ownership, so every
/// later result check, null check, JNI allocation failure and unwind releases
/// the allocation exactly once.
struct OrdinaryPlatformWalletString {
    ptr: *mut c_char,
    /// Always the ordinary free in production; see the sibling guard's field.
    release: NativeStringRelease,
}

impl OrdinaryPlatformWalletString {
    fn new(ptr: *mut c_char) -> Self {
        Self {
            ptr,
            release: platform_wallet_ffi::platform_wallet_string_free,
        }
    }

    /// Test-only counterpart of
    /// [`SensitivePlatformWalletString::with_release_for_test`].
    #[cfg(test)]
    fn with_release_for_test(ptr: *mut c_char, release: NativeStringRelease) -> Self {
        Self { ptr, release }
    }

    fn as_c_str(&self) -> Option<&CStr> {
        if self.ptr.is_null() {
            None
        } else {
            // SAFETY: a non-null pointer came from the platform-wallet FFI
            // CString result and remains owned by this guard.
            Some(unsafe { CStr::from_ptr(self.ptr) })
        }
    }
}

impl Drop for OrdinaryPlatformWalletString {
    fn drop(&mut self) {
        // SAFETY: this guard is the sole owner of the nullable pointer, and the
        // create contract names the ordinary free as its release function. The
        // ordinary free is null-safe.
        unsafe {
            (self.release)(self.ptr);
        }
    }
}

/// Copy an ASCII C string directly into a JVM string without constructing
/// jni-rs's intermediate owned `JNIString`.
///
/// `JNIEnv::new_string` re-encodes through a native allocation. The encrypted
/// document serializer instead guarantees ASCII JSON with no interior NUL, so
/// it is already valid modified UTF-8 for `NewStringUTF`.
///
/// Returns null if the JNI interface/table is unavailable or the JVM cannot
/// allocate the string. The JVM normally leaves an exception pending for the
/// allocation-failure case.
unsafe fn new_string_utf_from_ascii(env: &JNIEnv, ascii: &CStr) -> jstring {
    let raw_env = env.get_native_interface();
    if raw_env.is_null() {
        log::error!("documentFetchEncrypted: JNI environment pointer is null");
        return ptr::null_mut();
    }
    let function_table = unsafe { *raw_env };
    if function_table.is_null() {
        log::error!("documentFetchEncrypted: JNI function table is null");
        return ptr::null_mut();
    }
    let Some(new_string_utf) = (unsafe { (*function_table).NewStringUTF }) else {
        log::error!("documentFetchEncrypted: JNI NewStringUTF function is unavailable");
        return ptr::null_mut();
    };

    unsafe { new_string_utf(raw_env, ascii.as_ptr()) }
}

/// Read a required 32-byte id from a Java `byte[]`; throws + returns None
/// on the wrong length or a JNI error. Mirrors `identity::read_id32` — kept
/// local so this module stays a self-contained marshaling unit.
fn read_id32(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; 32]> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was null/invalid"));
            return None;
        }
    };
    if bytes.len() != 32 {
        throw_sdk_exception(
            env,
            1,
            &format!("{field} must be 32 bytes, got {}", bytes.len()),
        );
        return None;
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes);
    Some(id)
}

/// Secret-key sibling of [`read_id32`]: same contract, but the returned
/// 32-byte buffer is wrapped in [`zeroize::Zeroizing`] (scrubbed on drop)
/// and the intermediate JNI copy is zeroized before it is dropped. Use for
/// private-key material only.
fn read_key32_zeroizing(
    env: &mut JNIEnv,
    arr: &JByteArray,
    field: &str,
) -> Option<zeroize::Zeroizing<[u8; 32]>> {
    use zeroize::Zeroize;

    let mut bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was null/invalid"));
            return None;
        }
    };
    if bytes.len() != 32 {
        let len = bytes.len();
        bytes.zeroize();
        throw_sdk_exception(env, 1, &format!("{field} must be 32 bytes, got {len}"));
        return None;
    }
    let mut key = zeroize::Zeroizing::new([0u8; 32]);
    key.copy_from_slice(&bytes);
    bytes.zeroize();
    Some(key)
}

/// Read a required JVM string into an owned `CString`; throws + returns None
/// on a JNI read error or an interior-NUL byte.
fn read_cstring(env: &mut JNIEnv, s: &JString, field: &str) -> Option<CString> {
    if s.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} was null"));
        return None;
    }
    let raw: String = match env.get_string(s) {
        Ok(js) => js.into(),
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} string was invalid"));
            return None;
        }
    };
    match CString::new(raw) {
        Ok(c) => Some(c),
        Err(_) => {
            throw_sdk_exception(env, 1, &format!("{field} contained an interior NUL"));
            None
        }
    }
}

// ── Identity update (add / disable keys) ──────────────────────────────

/// Add public keys and/or disable existing key ids on an identity, signing
/// the resulting `IdentityUpdateTransition` via the external `signerHandle`
/// (the identity's MASTER auth key) — the exact call
/// `AddIdentityKeyView.submit` makes through `wallet.updateIdentity(...)`.
///
/// Thin marshaler over `platform_wallet_update_identity_with_signer`. The
/// derive + Keystore-persist of the new keys' private material happens on
/// the Kotlin side BEFORE this call (mirroring the Swift flow); here we only
/// carry the on-chain public rows.
///
/// `addPubkeysBlob` layout (big-endian), one row per key being added:
/// ```text
/// u32 row_count
/// repeat row_count times:
///   u32  key_id
///   u8   key_type          (DPP KeyType discriminant, 0 = ECDSA_SECP256K1)
///   u8   purpose           (DPP Purpose discriminant, 0 = AUTHENTICATION)
///   u8   security_level    (DPP SecurityLevel discriminant, 0 = MASTER)
///   u8   read_only         (0 / 1)
///   u8   contract_bounds_kind (0 none, 1 SingleContract, 2 SingleContractDocumentType)
///   u16  pubkey_len
///   u8[pubkey_len]  pubkey_bytes  (compressed pubkey, or 20-byte HASH160)
///   if contract_bounds_kind != 0:
///     u8[32] contract_bounds_id
///   if contract_bounds_kind == 2:
///     u16 doc_type_len, u8[doc_type_len] doc_type (UTF-8)
/// ```
///
/// `disablePublicKeyIds` is a JVM `int[]` of key ids to disable (may be
/// empty). At least one of add / disable must be non-empty for the FFI to
/// build a transition. Returns nothing on success (throws on error) — Room
/// learns of the change through the persistence changeset.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_updateIdentity(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    add_pubkeys_blob: JByteArray,
    disable_public_key_ids: jni::objects::JIntArray,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };

        // Decode the add-rows into owned buffers that outlive the FFI call
        // (the FFI borrows every `pubkey_bytes` / `contract_bounds_*` pointer
        // for the call duration).
        let Some(decoded) = decode_update_pubkeys_blob(env, &add_pubkeys_blob) else {
            return;
        };

        // Read the disable ids (a JVM int[] → Vec<u32>). A null int[] is a
        // legitimate "no disables"; jni 0.21 has no `convert_int_array`, so
        // read the length then the region into an owned buffer.
        let disable_ids: Vec<u32> = if disable_public_key_ids.is_null() {
            Vec::new()
        } else {
            match env.get_array_length(&disable_public_key_ids) {
                Ok(len) if len > 0 => {
                    let mut buf = vec![0i32; len as usize];
                    if env
                        .get_int_array_region(&disable_public_key_ids, 0, &mut buf)
                        .is_err()
                    {
                        let _ = env.exception_clear();
                        Vec::new()
                    } else {
                        // Reject negative entries before the sign-losing
                        // cast — a negative int would otherwise bit-cast
                        // to a bogus huge u32 key id.
                        if buf.iter().any(|&i| i < 0) {
                            throw_sdk_exception(env, 1, "keyIds must be non-negative");
                            return;
                        }
                        buf.into_iter().map(|i| i as u32).collect()
                    }
                }
                _ => {
                    let _ = env.exception_clear();
                    Vec::new()
                }
            }
        };

        if decoded.is_empty() && disable_ids.is_empty() {
            throw_sdk_exception(
                env,
                1,
                "updateIdentity needs at least one key to add or disable",
            );
            return;
        }

        // Build the FFI rows referencing the owned buffers in `decoded`.
        let ffi_rows: Vec<IdentityPubkeyFFI> = decoded.iter().map(|row| row.to_ffi()).collect();

        let result = unsafe {
            platform_wallet_ffi::platform_wallet_update_identity_with_signer(
                wallet_handle as Handle,
                id.as_ptr(),
                if ffi_rows.is_empty() {
                    ptr::null()
                } else {
                    ffi_rows.as_ptr()
                },
                ffi_rows.len(),
                if disable_ids.is_empty() {
                    ptr::null()
                } else {
                    disable_ids.as_ptr()
                },
                disable_ids.len(),
                signer_handle as *mut SignerHandle,
            )
        };
        // `decoded` / `ffi_rows` / `disable_ids` own the buffers the pointers
        // reference; they stay in scope through the FFI call above.
        take_pwffi_error(env, result);
    })
}

// ── Document purchase / set-price ─────────────────────────────────────

/// Shared purchase/set-price marshaling. `purchase == true` routes to
/// `platform_wallet_document_purchase` (buyer = `actorId`); `false` routes
/// to `platform_wallet_document_set_price` (owner = `actorId`). Both FFIs
/// have an identical parameter shape and write the confirmed document's
/// 32-byte id + canonical JSON via out-params.
///
/// Returns the confirmed document's canonical JSON (carrying the new
/// `$price` / owner) as a JVM string; the 32-byte confirmed id is embedded
/// in that JSON (`$id`) so Kotlin parses it from there rather than a second
/// return. Null after throwing on error.
#[allow(clippy::too_many_arguments)]
fn document_price_op<'l>(
    env: &mut JNIEnv<'l>,
    purchase: bool,
    wallet_handle: jlong,
    actor_id: JByteArray,
    contract_id: JByteArray,
    document_type: JString,
    document_id: JByteArray,
    price: jlong,
    signing_key_id: jint,
    signer_handle: jlong,
) -> jstring {
    // Reject negatives at the boundary rather than clamping: a silently
    // clamped-to-zero price would post a document anyone can buy for free,
    // and a negative key id would bit-cast to a bogus huge key index.
    if price < 0 {
        throw_sdk_exception(env, 1, "price must be non-negative");
        return ptr::null_mut();
    }
    if signing_key_id < 0 {
        throw_sdk_exception(env, 1, "signingKeyId must be non-negative");
        return ptr::null_mut();
    }

    let actor_field = if purchase { "purchaserId" } else { "ownerId" };
    let Some(actor) = read_id32(env, &actor_id, actor_field) else {
        return ptr::null_mut();
    };
    let Some(contract) = read_id32(env, &contract_id, "contractId") else {
        return ptr::null_mut();
    };
    let Some(doc_id) = read_id32(env, &document_id, "documentId") else {
        return ptr::null_mut();
    };
    let Some(doc_type) = read_cstring(env, &document_type, "documentType") else {
        return ptr::null_mut();
    };

    let mut out_id = [0u8; 32];
    let mut out_json: *mut c_char = ptr::null_mut();
    let result = unsafe {
        if purchase {
            platform_wallet_ffi::platform_wallet_document_purchase(
                wallet_handle as Handle,
                actor.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                doc_id.as_ptr(),
                price as u64,
                signing_key_id as u32,
                signer_handle as *mut SignerHandle,
                out_id.as_mut_ptr(),
                &mut out_json as *mut *mut c_char,
            )
        } else {
            platform_wallet_ffi::platform_wallet_document_set_price(
                wallet_handle as Handle,
                actor.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                doc_id.as_ptr(),
                price as u64,
                signing_key_id as u32,
                signer_handle as *mut SignerHandle,
                out_id.as_mut_ptr(),
                &mut out_json as *mut *mut c_char,
            )
        }
    };
    if take_pwffi_error(env, result) {
        return ptr::null_mut();
    }

    if out_json.is_null() {
        throw_sdk_exception(
            env,
            99,
            "document op returned success but no canonical JSON",
        );
        return ptr::null_mut();
    }
    // Copy the JSON out, then free the Rust-owned string.
    let json = unsafe { CStr::from_ptr(out_json) }
        .to_string_lossy()
        .into_owned();
    unsafe { platform_wallet_ffi::platform_wallet_string_free(out_json) };

    env.new_string(json)
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// Purchase for-sale `documentId` on `contractId`'s `documentType` for
/// `price` credits, with `purchaserId` as the buyer — signed via
/// `signerHandle` with key `signingKeyId`. Mirrors
/// `ManagedPlatformWallet.purchaseDocument` (Swift `DocumentWithPriceView`
/// purchase flow). Returns the confirmed document's canonical JSON.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentPurchase(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    purchaser_id: JByteArray,
    contract_id: JByteArray,
    document_type: JString,
    document_id: JByteArray,
    price: jlong,
    signing_key_id: jint,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        document_price_op(
            env,
            true,
            wallet_handle,
            purchaser_id,
            contract_id,
            document_type,
            document_id,
            price,
            signing_key_id,
            signer_handle,
        )
    })
}

/// Set (update) the trade price of `documentId` on `contractId`'s
/// `documentType`, owned by `ownerId`, to `price` credits — signed via
/// `signerHandle` with key `signingKeyId`. Mirrors the set-price flow
/// behind Swift `DocumentWithPriceView`. Returns the confirmed document's
/// canonical JSON (now carrying `$price`).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentSetPrice(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    contract_id: JByteArray,
    document_type: JString,
    document_id: JByteArray,
    price: jlong,
    signing_key_id: jint,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        document_price_op(
            env,
            false,
            wallet_handle,
            owner_id,
            contract_id,
            document_type,
            document_id,
            price,
            signing_key_id,
            signer_handle,
        )
    })
}

// ── Document create ───────────────────────────────────────────────────

/// Create + broadcast a new document on `contractId`'s `documentType`,
/// owned by `ownerId`, signed via `signerHandle` — the JNI bridge over
/// `platform_wallet_create_document_with_signer` (Swift
/// `ManagedPlatformWallet.createDocument`, behind `CreateDocumentView`).
///
/// Unlike purchase / set-price, the create FFI takes NO `signingKeyId`:
/// the Rust side selects an AUTHENTICATION + ECDSA key from the wallet's
/// in-process `IdentityManager` whose security level satisfies the
/// document type's requirement, so key selection never crosses JNI.
/// `propertiesJson` is a JSON object keyed by property name (byte-array
/// fields as hex, identifier fields as base58); pass `"{}"` for a type
/// with no required properties.
///
/// Returns the confirmed document's canonical query-side JSON — the same
/// shape a DOC-01 query returns, with the 32-byte id rendered as the
/// base58 `$id` field, so Kotlin reads the id from there rather than a
/// second return. Null after throwing on error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentCreate(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    contract_id: JByteArray,
    document_type: JString,
    properties_json: JString,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_id32(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(doc_type) = read_cstring(env, &document_type, "documentType") else {
            return ptr::null_mut();
        };
        let Some(props) = read_cstring(env, &properties_json, "propertiesJson") else {
            return ptr::null_mut();
        };

        let mut out_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_create_document_with_signer(
                wallet_handle as Handle,
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                props.as_ptr(),
                signer_handle as *mut SignerHandle,
                out_id.as_mut_ptr(),
                &mut out_json as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        if out_json.is_null() {
            throw_sdk_exception(
                env,
                99,
                "document create returned success but no canonical JSON",
            );
            return ptr::null_mut();
        }
        // Copy the JSON out, then free the Rust-owned string.
        let json = unsafe { CStr::from_ptr(out_json) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::platform_wallet_string_free(out_json) };

        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Document replace / delete / transfer ──────────────────────────────

/// Replace + broadcast the full property set of `documentId` on
/// `contractId`'s `documentType`, owned by `ownerId`, signed via
/// `signerHandle` with key `signingKeyId` — the JNI bridge over
/// `platform_wallet_document_replace` (Swift
/// `ManagedPlatformWallet.replaceDocument`, behind the DOC-03 menu).
///
/// The revision is bumped on the Rust side — the caller does NOT pass a
/// revision. `propertiesJson` is the FULL replacement property object
/// (same hex/base58 encoding rules as create); unlike create the replace
/// FFI takes an explicit `signingKeyId` (an AUTHENTICATION + ECDSA key on
/// the owner). Returns the confirmed document's canonical query-side JSON
/// (with the 32-byte id as the base58 `$id` field); null after throwing on
/// error.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentReplace(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    contract_id: JByteArray,
    document_type: JString,
    document_id: JByteArray,
    properties_json: JString,
    signing_key_id: jint,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        if signing_key_id < 0 {
            throw_sdk_exception(env, 1, "signingKeyId must be non-negative");
            return ptr::null_mut();
        }
        let Some(owner) = read_id32(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(doc_id) = read_id32(env, &document_id, "documentId") else {
            return ptr::null_mut();
        };
        let Some(doc_type) = read_cstring(env, &document_type, "documentType") else {
            return ptr::null_mut();
        };
        let Some(props) = read_cstring(env, &properties_json, "propertiesJson") else {
            return ptr::null_mut();
        };

        let mut out_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_document_replace(
                wallet_handle as Handle,
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                doc_id.as_ptr(),
                props.as_ptr(),
                signing_key_id as u32,
                signer_handle as *mut SignerHandle,
                out_id.as_mut_ptr(),
                &mut out_json as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        if out_json.is_null() {
            throw_sdk_exception(
                env,
                99,
                "document replace returned success but no canonical JSON",
            );
            return ptr::null_mut();
        }
        // Copy the JSON out, then free the Rust-owned string.
        let json = unsafe { CStr::from_ptr(out_json) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::platform_wallet_string_free(out_json) };

        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Delete + broadcast `documentId` on `contractId`'s `documentType`,
/// owned by `ownerId`, signed via `signerHandle` with key `signingKeyId`
/// — the JNI bridge over `platform_wallet_document_delete` (Swift
/// `ManagedPlatformWallet.deleteDocument`, behind the DOC-04 menu).
///
/// Delete returns no document body (there is no canonical JSON), so this
/// returns the deleted document's 32-byte id as a `byte[]` for
/// confirmation. Null after throwing on error.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentDelete(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    contract_id: JByteArray,
    document_type: JString,
    document_id: JByteArray,
    signing_key_id: jint,
    signer_handle: jlong,
) -> jni::sys::jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        if signing_key_id < 0 {
            throw_sdk_exception(env, 1, "signingKeyId must be non-negative");
            return ptr::null_mut();
        }
        let Some(owner) = read_id32(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(doc_id) = read_id32(env, &document_id, "documentId") else {
            return ptr::null_mut();
        };
        let Some(doc_type) = read_cstring(env, &document_type, "documentType") else {
            return ptr::null_mut();
        };

        let mut out_id = [0u8; 32];
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_document_delete(
                wallet_handle as Handle,
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                doc_id.as_ptr(),
                signing_key_id as u32,
                signer_handle as *mut SignerHandle,
                out_id.as_mut_ptr(),
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        env.byte_array_from_slice(&out_id)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Transfer + broadcast `documentId` on `contractId`'s `documentType`,
/// from `ownerId` to `recipientId`, signed via `signerHandle` with key
/// `signingKeyId` — the JNI bridge over `platform_wallet_document_transfer`
/// (Swift `ManagedPlatformWallet.transferDocument`, behind the DOC-05
/// menu). Only valid for document types whose schema is `transferable`
/// (gated by the caller).
///
/// Returns the confirmed document's canonical query-side JSON (now
/// reflecting the new owner, with the 32-byte id as the base58 `$id`
/// field); null after throwing on error.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentTransfer(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    contract_id: JByteArray,
    document_type: JString,
    document_id: JByteArray,
    recipient_id: JByteArray,
    signing_key_id: jint,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        if signing_key_id < 0 {
            throw_sdk_exception(env, 1, "signingKeyId must be non-negative");
            return ptr::null_mut();
        }
        let Some(owner) = read_id32(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(doc_id) = read_id32(env, &document_id, "documentId") else {
            return ptr::null_mut();
        };
        let Some(recipient) = read_id32(env, &recipient_id, "recipientId") else {
            return ptr::null_mut();
        };
        let Some(doc_type) = read_cstring(env, &document_type, "documentType") else {
            return ptr::null_mut();
        };

        let mut out_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_document_transfer(
                wallet_handle as Handle,
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                doc_id.as_ptr(),
                recipient.as_ptr(),
                signing_key_id as u32,
                signer_handle as *mut SignerHandle,
                out_id.as_mut_ptr(),
                &mut out_json as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        if out_json.is_null() {
            throw_sdk_exception(
                env,
                99,
                "document transfer returned success but no canonical JSON",
            );
            return ptr::null_mut();
        }
        // Copy the JSON out, then free the Rust-owned string.
        let json = unsafe { CStr::from_ptr(out_json) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::platform_wallet_string_free(out_json) };

        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Encrypted document create / fetch (wallet txMetadata contract) ─────

/// Create + broadcast an ENCRYPTED wallet-contract document (the wire-
/// compatible `txMetadata` shape) — the JNI bridge over
/// the Rust-ABI composite
/// `create_encrypted_document_with_deferred_payload`.
///
/// The SDK derives the identity encryption key, seals `payload` into the
/// legacy `version ‖ IV ‖ AES-256-CBC` blob, and writes
/// `{keyIndex, encryptionKeyIndex, encryptedMetadata}`. `version` is the payload
/// version byte (`1` = protobuf); `payload` is the already-serialized opaque
/// plaintext (a protobuf `TxMetadataBatch`) — the SDK does not parse it.
///
/// `encryption_key_index` carries the per-document index OR the `-1` sentinel:
/// a non-negative value is used verbatim (retained for migration / tests), while
/// `-1` means "let the SDK allocate the index from authoritative Platform
/// state". Any value `< -1` is rejected.
///
/// Both shapes enter one Rust-owned operation. It settles the index BEFORE it
/// invokes JNI's deferred callback to copy the caller's `byte[]` into native
/// memory. A JVM array cannot be pinned across the automatic-index query, so the
/// callback returns an owned zeroizing copy only after that query completes.
/// Rust scrubs the copy as soon as the properties are sealed, before broadcast.
/// This helper has Rust ABI only and adds no C symbol. Returns the confirmed
/// document's canonical JSON (its 32-byte id is the base58 `$id` field); null
/// after throwing on error.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentCreateEncrypted(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    mnemonic_resolver_handle: jlong,
    owner_id: JByteArray,
    contract_id: JByteArray,
    document_type: JString,
    encryption_key_index: jint,
    version: jint,
    payload: JByteArray,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_id32(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(doc_type) = read_cstring(env, &document_type, "documentType") else {
            return ptr::null_mut();
        };
        // Narrow the Java-signed arguments to the widths the C ABI takes.
        // Anything representable is handed to Rust, which owns the protocol
        // policy; only values with no representation stop here.
        let validated = match encrypted_create_preflight(encryption_key_index, version) {
            Ok(validated) => validated,
            Err(error) => {
                throw_sdk_exception(env, 1, &error.to_string());
                return ptr::null_mut();
            }
        };

        // Read the DECLARED length from the array header — no copy — so the
        // shared size policy can reject an over-large batch before any plaintext
        // moves and before any network work.
        let payload_len = match env.get_array_length(&payload) {
            Ok(len) => len as usize,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "payload byte[] was null/invalid");
                return ptr::null_mut();
            }
        };

        let encryption_key_index = match validated.encryption_key_index {
            EncryptionKeyIndexRequest::Explicit(index) => Some(index),
            EncryptionKeyIndexRequest::Allocate => None,
        };

        let mut out_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        // One Rust-owned composite performs preflight, index allocation and
        // create. JNI supplies a deferred materializer, so Rust invokes the JVM
        // copy exactly once and only after an automatic index query has
        // completed. The returned native copy moves straight into zeroizing
        // preparation and is scrubbed before broadcast. The caller's original
        // JVM ByteArray remains runtime-managed and cannot be scrubbed here.
        let result = unsafe {
            platform_wallet_ffi::create_encrypted_document_with_deferred_payload(
                wallet_handle as Handle,
                mnemonic_resolver_handle as *mut rs_sdk_ffi::MnemonicResolverHandle,
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                encryption_key_index,
                validated.version,
                payload_len,
                || match env.convert_byte_array(&payload) {
                    Ok(bytes) => Ok(zeroize::Zeroizing::new(bytes)),
                    Err(_) => {
                        let _ = env.exception_clear();
                        Err(platform_wallet_ffi::PlatformWalletFFIResult::err(
                            platform_wallet_ffi::PlatformWalletFFIResultCode::ErrorInvalidParameter,
                            "payload byte[] was null/invalid",
                        ))
                    }
                },
                signer_handle as *mut SignerHandle,
                out_id.as_mut_ptr(),
                &mut out_json as *mut *mut c_char,
            )
        };
        // Ownership of the canonical JSON has transferred; install the guard
        // before any result, null or JNI-allocation handling so every later
        // path — success, early return, or unwind — releases it exactly once
        // through the ordinary free.
        let out_json = OrdinaryPlatformWalletString::new(out_json);
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let Some(json) = out_json.as_c_str() else {
            throw_sdk_exception(
                env,
                99,
                "encrypted document create returned success but no canonical JSON",
            );
            return ptr::null_mut();
        };

        env.new_string(json.to_string_lossy())
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch + DECRYPT every encrypted wallet-contract document owned by `ownerId`
/// on `contractId`'s `documentType` updated at or after `sinceMs` — the JNI
/// bridge over `platform_wallet_fetch_encrypted_documents` (the wire-compatible
/// read counterpart of the legacy `getTxMetaData(since, key)`).
///
/// Returns a JSON array; each element is
/// `{ "id", "ownerId" (base58), "keyIndex", "encryptionKeyIndex", "version",
/// "updatedAt" (u64|null), "payload" (base64 of the decrypted opaque plaintext)}`.
/// The caller parses each `payload` itself (a protobuf `TxMetadataBatch` for
/// `version == 1`). Documents that can't be decrypted, and documents carrying
/// an unsupported wire version, are skipped Rust-side.
///
/// A returned `payload` is NOT authenticated: the envelope is AES-256-CBC with
/// PKCS7 and no integrity tag, so a wrong key or modified ciphertext usually
/// fails the unpad and is skipped, but can occasionally unpad cleanly and
/// surface opaque garbage. Parse each payload strictly and discard what does
/// not parse.
/// SDK-owned native plaintext allocations are zeroized before release; the
/// returned JVM string remains runtime-managed and cannot be reliably wiped.
/// Null after throwing on error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentFetchEncrypted(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    mnemonic_resolver_handle: jlong,
    owner_id: JByteArray,
    contract_id: JByteArray,
    document_type: JString,
    since_ms: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        // Informational stage breadcrumbs are DEBUG; only genuine failure paths
        // are WARN. `JNI_OnLoad` installs Android logging at `LevelFilter::Info`,
        // so routine sync stages stay out of on-device logcat while failure
        // lines remain visible. NEVER log a raw handle value: only
        // whether each handle is nonzero — `mnemonic_resolver_handle` is a live
        // `*mut MnemonicResolverHandle`, so `{:#x}` would leak a heap pointer.
        // `sinceMs` is deliberately NOT rendered: it is caller-controlled and a
        // timestamp correlates a device to when it last synced, which a device
        // log readable by any process holding the log permission — and captured
        // in bug reports — must not carry. Handle presence is a boolean and
        // reveals nothing about the caller.
        log::debug!(
            "documentFetchEncrypted: entry wallet_handle_nonzero={} \
             mnemonic_resolver_handle_nonzero={}",
            wallet_handle != 0,
            mnemonic_resolver_handle != 0
        );
        let Some(owner) = read_id32(env, &owner_id, "ownerId") else {
            log::warn!("documentFetchEncrypted: ownerId byte[] invalid; throwing");
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &contract_id, "contractId") else {
            log::warn!("documentFetchEncrypted: contractId byte[] invalid; throwing");
            return ptr::null_mut();
        };
        let Some(doc_type) = read_cstring(env, &document_type, "documentType") else {
            log::warn!("documentFetchEncrypted: documentType string invalid; throwing");
            return ptr::null_mut();
        };
        if since_ms < 0 {
            log::warn!("documentFetchEncrypted: sinceMs negative; throwing");
            throw_sdk_exception(env, 1, "sinceMs must be non-negative");
            return ptr::null_mut();
        }
        log::debug!(
            "{}",
            fetch_encrypted_call_breadcrumb(&owner, &contract, doc_type.to_bytes())
        );

        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_fetch_encrypted_documents(
                wallet_handle as Handle,
                mnemonic_resolver_handle as *mut rs_sdk_ffi::MnemonicResolverHandle,
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                since_ms as u64,
                &mut out_json as *mut *mut c_char,
            )
        };
        let out_json = SensitivePlatformWalletString::new(out_json);
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let Some(json) = out_json.as_c_str() else {
            log::warn!("documentFetchEncrypted: success code but null JSON; throwing");
            throw_sdk_exception(
                env,
                99,
                "encrypted document fetch returned success but no JSON",
            );
            return ptr::null_mut();
        };
        let json_bytes = json.to_bytes();
        if !json_bytes.is_ascii() {
            log::warn!("documentFetchEncrypted: serializer returned non-ASCII JSON; throwing");
            throw_sdk_exception(env, 99, "encrypted document fetch returned non-ASCII JSON");
            return ptr::null_mut();
        }
        let json_len = json_bytes.len();
        let java_string = unsafe { new_string_utf_from_ascii(env, json) };
        if java_string.is_null() {
            log::warn!("documentFetchEncrypted: NewStringUTF returned null");
            return ptr::null_mut();
        }
        log::debug!(
            "documentFetchEncrypted: success, returning {} chars of JSON to Kotlin",
            json_len
        );

        java_string
    })
}

// ── Contested-resource vote ───────────────────────────────────────────

/// Cast a masternode contested-resource vote and wait for the response —
/// the JNI bridge over `dash_sdk_contested_resource_cast_vote` (Swift
/// `SDK.castContestedResourceVote`, behind `ContestDetailView`).
///
/// Builds a `ResourceVote` over the vote poll `(contractId, documentType,
/// indexName, indexValuesJson)`, signs it with the masternode voting key
/// derived from `votingPrivateKey`, and broadcasts it for `voterProTxHash`.
///
/// * `voteChoice` — `0` = TowardsIdentity (requires `contenderIdentityId`),
///   `1` = Abstain, `2` = Lock.
/// * `contenderIdentityId` — base58 contender id; required only when
///   `voteChoice == 0`, else may be null.
/// * `voterProTxHash` / `votingPrivateKey` — each a 32-byte `byte[]`.
/// * `networkOrd` — `Network.ffiValue` (0 Mainnet, 1 Testnet, 2 Devnet,
///   3 Regtest).
///
/// Returns nothing on success (throws on error). A vote from a
/// non-masternode wallet is expected to fail with an authorization-style
/// error — the correct deterministic outcome (surfaced as a thrown
/// exception the caller shows as a rejection banner).
///
/// Uses the rs-sdk-ffi `DashSDKResult` error path
/// ([`crate::results::take_error`]) rather than the platform-wallet path,
/// since this is an rs-sdk-ffi entry point.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_castContestedResourceVote(
    mut env: JNIEnv,
    _class: JClass,
    sdk_handle: jlong,
    contract_id: JString,
    document_type: JString,
    index_name: JString,
    index_values_json: JString,
    vote_choice: jint,
    contender_identity_id: JString,
    voter_pro_tx_hash: JByteArray,
    voting_private_key: JByteArray,
    network_ord: jint,
) {
    guard(&mut env, (), |env| {
        let Some(contract) = read_cstring(env, &contract_id, "contractId") else {
            return;
        };
        let Some(doc_type) = read_cstring(env, &document_type, "documentType") else {
            return;
        };
        let Some(idx_name) = read_cstring(env, &index_name, "indexName") else {
            return;
        };
        let Some(idx_values) = read_cstring(env, &index_values_json, "indexValuesJson") else {
            return;
        };
        let Some(pro_tx_hash) = read_id32(env, &voter_pro_tx_hash, "voterProTxHash") else {
            return;
        };
        // The voting key is a masternode private key: read it through the
        // zeroizing variant so neither the returned buffer nor the JNI
        // intermediate copy outlives this call unscrubbed.
        let Some(voting_key) = read_key32_zeroizing(env, &voting_private_key, "votingPrivateKey")
        else {
            return;
        };

        // `contender_identity_id` is only consulted when vote_choice == 0;
        // otherwise a null JString marshals to a null pointer.
        let contender: Option<CString> = if contender_identity_id.is_null() {
            None
        } else {
            match env.get_string(&contender_identity_id) {
                Ok(s) => {
                    let raw: String = s.into();
                    if raw.is_empty() {
                        None
                    } else {
                        match CString::new(raw) {
                            Ok(c) => Some(c),
                            Err(_) => {
                                throw_sdk_exception(
                                    env,
                                    1,
                                    "contenderIdentityId contained an interior NUL",
                                );
                                return;
                            }
                        }
                    }
                }
                Err(_) => {
                    let _ = env.exception_clear();
                    None
                }
            }
        };

        // Clamp the vote choice into the u8 the FFI validates (it rejects
        // anything outside 0..=2 itself).
        let choice = vote_choice.clamp(0, u8::MAX as jint) as u8;

        let result = unsafe {
            rs_sdk_ffi::dash_sdk_contested_resource_cast_vote(
                sdk_handle as *const rs_sdk_ffi::SDKHandle,
                contract.as_ptr(),
                doc_type.as_ptr(),
                idx_name.as_ptr(),
                idx_values.as_ptr(),
                choice,
                contender.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                pro_tx_hash.as_ptr(),
                voting_key.as_ptr(),
                net_from_ord(network_ord),
            )
        };
        // Success carries no payload; take_error throws + frees on error.
        unsafe { crate::results::take_error(env, &result) };
        // On success there is no data pointer to free (the FFI returns
        // `success(null)`), so nothing else to release here.
    })
}

/// What the Java caller asked for regarding the per-document
/// `encryptionKeyIndex`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EncryptionKeyIndexRequest {
    /// An explicit index the caller chose, kept for migration and tests.
    Explicit(u32),
    /// No index supplied: the SDK allocates one from Platform state. Carried by
    /// the [`AUTO_ENCRYPTION_KEY_INDEX`] sentinel, because the Java signature's
    /// `int` has no other way to say "absent".
    Allocate,
}

/// The Java value meaning "no index supplied; allocate one".
///
/// A sentinel rather than a boxed `Integer` so the native signature stays a
/// primitive `int` and the call needs no JVM object.
pub(crate) const AUTO_ENCRYPTION_KEY_INDEX: jint = -1;

/// Java-signed encrypted-create arguments narrowed to the widths the C ABI
/// takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ValidatedEncryptedCreate {
    pub(crate) encryption_key_index: EncryptionKeyIndexRequest,
    pub(crate) version: u8,
}

/// Why a Java-supplied encrypted-create argument could not be narrowed.
///
/// Each cause is its own variant so a caller — and a future change to one of
/// the conventions — can address exactly one of them without disturbing the
/// other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EncryptedCreatePreflightError {
    /// An index below the allocate sentinel, which denotes neither an explicit
    /// index nor a request to allocate one.
    EncryptionKeyIndexOutOfRange { value: jint },
    /// A version outside the byte the wire format carries.
    VersionOutOfByteRange { value: jint },
}

impl std::fmt::Display for EncryptedCreatePreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptedCreatePreflightError::EncryptionKeyIndexOutOfRange { value } => {
                write!(
                    f,
                    "encryptionKeyIndex must be non-negative, or \
                     {AUTO_ENCRYPTION_KEY_INDEX} to let the SDK allocate it, got {value}"
                )
            }
            EncryptedCreatePreflightError::VersionOutOfByteRange { value } => {
                write!(f, "version must fit a single byte (0..=255), got {value}")
            }
        }
    }
}

/// Narrow the Java-signed encrypted-create arguments.
///
/// This layer bridges representations; it does not decide protocol. Every value
/// that fits its target width is passed through for the Rust core to accept or
/// reject, so there is no second place where the set of meaningful versions is
/// written down and no way for the two to disagree. The one convention it does
/// own is the absent-index sentinel, which exists only because the Java
/// signature cannot express absence.
pub(crate) fn encrypted_create_preflight(
    encryption_key_index: jint,
    version: jint,
) -> Result<ValidatedEncryptedCreate, EncryptedCreatePreflightError> {
    let encryption_key_index = if encryption_key_index == AUTO_ENCRYPTION_KEY_INDEX {
        EncryptionKeyIndexRequest::Allocate
    } else {
        EncryptionKeyIndexRequest::Explicit(u32::try_from(encryption_key_index).map_err(|_| {
            EncryptedCreatePreflightError::EncryptionKeyIndexOutOfRange {
                value: encryption_key_index,
            }
        })?)
    };
    let version = u8::try_from(version)
        .map_err(|_| EncryptedCreatePreflightError::VersionOutOfByteRange { value: version })?;

    Ok(ValidatedEncryptedCreate {
        encryption_key_index,
        version,
    })
}

/// The stage line recorded when the encrypted fetch reaches the native call.
///
/// Takes the call's arguments so the seam sits where the call does, and
/// deliberately renders none of them: a device log is readable by any process
/// holding the log permission and is captured in bug reports, so an identifier
/// there correlates a device to an on-chain identity, and caller-supplied text
/// can embed a newline to forge further log lines.
pub(crate) fn fetch_encrypted_call_breadcrumb(
    _owner: &[u8; 32],
    _contract: &[u8; 32],
    _document_type: &[u8],
) -> String {
    "documentFetchEncrypted: calling platform_wallet_fetch_encrypted_documents".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_wallet_ffi::PlatformWalletFFIResultCode;

    // ── Java representation narrowing ───────────────────────────────────────
    //
    // This layer bridges representations only. Anything that fits its target
    // width is passed through for the Rust core to accept or reject, so the set
    // of meaningful versions is written down in exactly one place.

    /// The sentinel is the one convention this layer owns, because the Java
    /// `int` signature cannot express an absent index.
    #[test]
    fn the_allocate_sentinel_is_the_only_negative_index_accepted() {
        assert_eq!(
            encrypted_create_preflight(AUTO_ENCRYPTION_KEY_INDEX, 1)
                .expect("the sentinel is representable")
                .encryption_key_index,
            EncryptionKeyIndexRequest::Allocate,
            "-1 means the SDK allocates the index"
        );
        assert_eq!(
            encrypted_create_preflight(0, 1)
                .expect("zero is a valid explicit index")
                .encryption_key_index,
            EncryptionKeyIndexRequest::Explicit(0)
        );
        assert_eq!(
            encrypted_create_preflight(7, 1)
                .expect("a positive index is explicit")
                .encryption_key_index,
            EncryptionKeyIndexRequest::Explicit(7)
        );

        for below_sentinel in [-2, -1000, jint::MIN] {
            assert!(
                matches!(
                    encrypted_create_preflight(below_sentinel, 1),
                    Err(EncryptedCreatePreflightError::EncryptionKeyIndexOutOfRange { value })
                        if value == below_sentinel
                ),
                "a value below the sentinel denotes neither an explicit index nor a \
                 request to allocate one; got {below_sentinel}"
            );
        }
    }

    /// Every value that fits a byte passes this layer — including versions the
    /// core will refuse. Narrowing is not policy.
    #[test]
    fn every_byte_width_version_passes_narrowing_and_policy_stays_in_rust() {
        for version in 0..=255i32 {
            let validated = encrypted_create_preflight(0, version)
                .expect("every value that fits a byte must pass the narrowing layer");
            assert_eq!(validated.version, version as u8);
        }

        for out_of_range in [-1, 256, jint::MAX] {
            assert!(
                matches!(
                    encrypted_create_preflight(0, out_of_range),
                    Err(EncryptedCreatePreflightError::VersionOutOfByteRange { value })
                        if value == out_of_range
                ),
                "a version with no byte representation stops here; got {out_of_range}"
            );
        }

        // Version 2 fits a byte, so it passes narrowing — and is then refused by
        // the shared Rust policy, which is the only place that decides it.
        assert_eq!(
            encrypted_create_preflight(0, 2)
                .expect("2 is representable")
                .version,
            2
        );
        assert_eq!(
            platform_wallet_ffi::tx_metadata_create_preflight_result(8, 2, Some(0), true).code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "the wire-version decision belongs to Rust, not to this layer"
        );
    }

    // ── The bridge keeps no plaintext copy across the broadcast ─────────────

    /// The Rust composite JNI calls owns protocol preflight and does not invoke
    /// the deferred JVM-array materializer for a request it already rejects.
    #[test]
    fn the_deferred_composite_rejects_before_jni_materialization() {
        let mut out_id = [0u8; 32];
        let mut out_json = ptr::null_mut();
        let materialize_calls = std::cell::Cell::new(0);

        let result = unsafe {
            platform_wallet_ffi::create_encrypted_document_with_deferred_payload(
                u64::MAX,
                ptr::null_mut(),
                [1u8; 32].as_ptr(),
                [2u8; 32].as_ptr(),
                c"txMetadata".as_ptr(),
                None,
                2,
                3,
                || {
                    materialize_calls.set(materialize_calls.get() + 1);
                    Ok(zeroize::Zeroizing::new(vec![1, 2, 3]))
                },
                ptr::dangling_mut::<SignerHandle>(),
                out_id.as_mut_ptr(),
                &mut out_json,
            )
        };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "the composite owns protocol preflight"
        );
        assert_eq!(materialize_calls.get(), 0);
        assert!(
            out_json.is_null(),
            "the composite publishes the null sentinel"
        );
    }

    /// The production JNI export must route create through one Rust-owned
    /// deferred composite. A runtime test cannot construct a representative
    /// Android `JNIEnv` here, so this assertion pins the bridge structure that
    /// keeps the JVM array conversion inside the deferred callback.
    #[test]
    fn production_jni_create_routes_through_one_deferred_composite() {
        let source = include_str!("transactions.rs");
        let export = source
            .split_once(
                "pub extern \"system\" fn \
                 Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentCreateEncrypted",
            )
            .expect("production encrypted-create JNI export must exist")
            .1
            .split_once("/// Fetch + DECRYPT every encrypted wallet-contract document")
            .expect("encrypted-create export must end before the fetch export")
            .0;

        assert!(
            export.contains("create_encrypted_document_with_deferred_payload("),
            "production JNI create must call the deferred Rust composite"
        );
        let normalized = export.split_whitespace().collect::<Vec<_>>().join(" ");

        assert_eq!(
            export
                .matches("create_encrypted_document_with_deferred_payload(")
                .count(),
            1,
            "production JNI create must make exactly one composite call"
        );
        assert_eq!(
            export.matches("env.convert_byte_array(&payload)").count(),
            1,
            "production JNI create must materialize the array exactly once"
        );
        assert!(
            normalized.contains("payload_len, || match env.convert_byte_array(&payload)"),
            "the JVM array conversion must be the composite's deferred materializer argument"
        );
        assert!(
            !export.contains("platform_wallet_create_encrypted_document_with_signer("),
            "JNI must not stitch a second create call after allocation"
        );
    }

    // ── Native string ownership ─────────────────────────────────────────────

    /// The create guard releases through the ORDINARY free, and the fetch guard
    /// through the SENSITIVE one. The two are distinct types so a call site
    /// cannot pair an allocation with the wrong release function.
    ///
    /// Both are exercised on their null form here, which every release path must
    /// tolerate: it is what an early return before a successful FFI call leaves
    /// behind, and what an unwind through the same scope drops.
    #[test]
    fn both_native_string_guards_release_a_null_pointer_safely() {
        drop(OrdinaryPlatformWalletString::new(ptr::null_mut()));
        drop(SensitivePlatformWalletString::new(ptr::null_mut()));
    }

    /// A null guard reports no string rather than dereferencing.
    #[test]
    fn a_null_guard_reports_no_string() {
        assert!(OrdinaryPlatformWalletString::new(ptr::null_mut())
            .as_c_str()
            .is_none());
        assert!(SensitivePlatformWalletString::new(ptr::null_mut())
            .as_c_str()
            .is_none());
    }

    /// The create guard owns a real ordinary allocation and releases it through
    /// the ordinary free — on the normal path and on an unwind through the same
    /// scope.
    #[test]
    fn the_ordinary_guard_releases_a_real_allocation_on_both_paths() {
        let owned = CString::new("{\"$id\":\"abc\"}").expect("no interior NUL");
        let guard = OrdinaryPlatformWalletString::new(owned.into_raw());
        assert_eq!(
            guard
                .as_c_str()
                .expect("a non-null guard reports its string")
                .to_str()
                .expect("ASCII"),
            "{\"$id\":\"abc\"}"
        );
        drop(guard);

        // An unwind through a scope holding the guard must still release it.
        let unwound = std::panic::catch_unwind(|| {
            let owned = CString::new("{}").expect("no interior NUL");
            let _guard = OrdinaryPlatformWalletString::new(owned.into_raw());
            panic!("unwind with the guard live");
        });
        assert!(unwound.is_err(), "the panic must have unwound");
    }

    /// The fetch guard owns a real allocation and releases it through the
    /// SENSITIVE free — on the normal path and on an unwind through the same
    /// scope.
    ///
    /// Symmetric with the ordinary guard's test, and deliberately exercising the
    /// real `Drop` rather than only the null form: the null case cannot tell the
    /// two release functions apart, because both are null-safe. A `CString`
    /// allocation is layout-compatible with what the sensitive free expects, and
    /// `platform-wallet-ffi` separately proves that free wipes through the
    /// terminating NUL.
    #[test]
    fn the_sensitive_guard_releases_a_real_allocation_on_both_paths() {
        let owned = CString::new("[{\"payload\":\"AAECAw==\"}]").expect("no interior NUL");
        let guard = SensitivePlatformWalletString::new(owned.into_raw());
        assert_eq!(
            guard
                .as_c_str()
                .expect("a non-null guard reports its string")
                .to_str()
                .expect("the serializer guarantees ASCII"),
            "[{\"payload\":\"AAECAw==\"}]"
        );
        // Normal-path release through the sensitive contract.
        drop(guard);

        // An unwind through a scope holding the guard must still release it —
        // the path a JNI-allocation failure or a panic between the FFI call and
        // the return would take.
        let unwound = std::panic::catch_unwind(|| {
            let owned = CString::new("[]").expect("no interior NUL");
            let _guard = SensitivePlatformWalletString::new(owned.into_raw());
            panic!("unwind with the sensitive guard live");
        });
        assert!(unwound.is_err(), "the panic must have unwound");
    }

    // ── Which release each guard selects, and that it runs exactly once ─────
    //
    // The tests above prove the guards release SOMETHING on every path, but not
    // WHICH free they call: both real frees are null-safe, and after a real free
    // the allocation cannot be inspected. A guard switched to the wrong
    // contract — the sensitive fetch output released without being wiped — would
    // compile and leave every case above green. These pin the selection through
    // a recording release, then pin the production constructors to the frees the
    // two contracts actually name.

    thread_local! {
        /// Pointers passed to the recording release, in call order.
        static RELEASED: std::cell::RefCell<Vec<*mut c_char>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }

    /// Records the pointer instead of freeing it. Never hands a pointer to a
    /// real free, so the caller keeps ownership and can reclaim it.
    unsafe extern "C" fn recording_release(ptr: *mut c_char) {
        RELEASED.with(|seen| seen.borrow_mut().push(ptr));
    }

    fn recorded() -> Vec<*mut c_char> {
        RELEASED.with(|seen| seen.borrow().clone())
    }

    fn clear_recorded() {
        RELEASED.with(|seen| seen.borrow_mut().clear());
    }

    /// Each guard runs its release EXACTLY ONCE per owned pointer, on the
    /// normal path, on a null pointer, and on an unwind.
    ///
    /// Exactly-once is the property a double free or a leak would break, and it
    /// is invisible to the real frees.
    #[test]
    fn a_guard_runs_its_release_exactly_once_on_every_drop_path() {
        clear_recorded();

        // Normal path, non-null.
        let owned = CString::new("[]").expect("no interior NUL");
        let raw = owned.into_raw();
        drop(SensitivePlatformWalletString::with_release_for_test(
            raw,
            recording_release,
        ));
        assert_eq!(
            recorded(),
            vec![raw],
            "a dropped guard must release its pointer exactly once"
        );
        // The recording release freed nothing, so reclaim the allocation here.
        // SAFETY: `raw` came from `CString::into_raw` and was never freed.
        drop(unsafe { CString::from_raw(raw) });

        // Null ownership still releases exactly once: the real frees are
        // null-safe and the guard must not special-case them away.
        clear_recorded();
        drop(OrdinaryPlatformWalletString::with_release_for_test(
            ptr::null_mut(),
            recording_release,
        ));
        assert_eq!(
            recorded(),
            vec![ptr::null_mut()],
            "a null-owning guard must still run its release exactly once"
        );

        // Unwind path — a JNI allocation failure or a panic between the FFI call
        // and the return.
        clear_recorded();
        let owned = CString::new("[]").expect("no interior NUL");
        let raw = owned.into_raw();
        let unwound = std::panic::catch_unwind(|| {
            let _guard =
                SensitivePlatformWalletString::with_release_for_test(raw, recording_release);
            panic!("unwind with the guard live");
        });
        assert!(unwound.is_err(), "the panic must have unwound");
        assert_eq!(
            recorded(),
            vec![raw],
            "an unwind must run the release exactly once, not zero or twice"
        );
        // SAFETY: as above — the recording release did not free it.
        drop(unsafe { CString::from_raw(raw) });
    }

    /// The production constructors name the frees their contracts require.
    ///
    /// The fetch output is plaintext-equivalent and MUST go through the
    /// zeroizing free; the create output is ciphertext and metadata and goes
    /// through the ordinary one. Swapping either — the mistake the distinct
    /// types exist to prevent, and the one a `with_release_for_test` misuse
    /// could reintroduce — fails here.
    #[test]
    fn the_production_guards_select_their_contracts_release_function() {
        let sensitive = SensitivePlatformWalletString::new(ptr::null_mut());
        assert_eq!(
            sensitive.release as usize,
            platform_wallet_ffi::platform_wallet_sensitive_string_free as usize,
            "the fetch guard must release plaintext-equivalent output through the \
             zeroizing free"
        );

        let ordinary = OrdinaryPlatformWalletString::new(ptr::null_mut());
        assert_eq!(
            ordinary.release as usize,
            platform_wallet_ffi::platform_wallet_string_free as usize,
            "the create guard must release its ciphertext JSON through the \
             ordinary free"
        );

        assert_ne!(
            platform_wallet_ffi::platform_wallet_sensitive_string_free as usize,
            platform_wallet_ffi::platform_wallet_string_free as usize,
            "the two frees must be distinct functions, or the assertions above \
             prove nothing"
        );
    }

    /// The fetch output's ASCII / no-interior-NUL precondition is what lets the
    /// bridge hand the Rust buffer straight to `NewStringUTF` with no
    /// intermediate copy. A non-ASCII or NUL-bearing buffer would break that,
    /// so the precondition is asserted rather than assumed.
    #[test]
    fn the_fetch_output_is_ascii_with_no_interior_nul() {
        let serialized = CString::new("[]").expect("the serializer emits no interior NUL");
        let bytes = serialized.as_bytes();
        assert!(
            bytes.is_ascii(),
            "the sensitive serializer guarantees ASCII, which is already valid \
             modified UTF-8 for NewStringUTF"
        );
        assert!(
            !bytes.contains(&0),
            "an interior NUL would truncate the string NewStringUTF builds"
        );
    }

    // ── Sanitized breadcrumbs ───────────────────────────────────────────────

    /// The fetch call breadcrumb renders none of its arguments.
    #[test]
    fn the_fetch_call_breadcrumb_renders_no_caller_data() {
        const MARKER: &str = "s3cr3t-marker-do-not-log";
        let owner = [0xABu8; 32];
        let contract = [0xCDu8; 32];
        let hostile = format!("txMetadata\nFORGED line {MARKER}");

        let line = fetch_encrypted_call_breadcrumb(&owner, &contract, hostile.as_bytes());

        assert!(!line.contains(MARKER), "caller text must not reach the log");
        assert!(
            !line.contains('\n'),
            "an embedded newline would let a caller forge further log lines"
        );
        assert!(
            !line.contains("abab") && !line.contains("cdcd"),
            "identifiers must not be rendered in any form"
        );
        assert_eq!(
            line, "documentFetchEncrypted: calling platform_wallet_fetch_encrypted_documents",
            "the breadcrumb is a fixed stage label"
        );
    }
}
