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

use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jint, jlong, jstring};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::identity_registration_with_signer::IdentityPubkeyFFI;
use rs_sdk_ffi::{FFINetwork, SignerHandle};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

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

/// FFINetwork ordinal → the crate's `FFINetwork` enum
/// (0=Mainnet, 2=Devnet, 3=Regtest, else Testnet). Matches
/// `identity::net_from_ord` and Kotlin's `Network.ffiValue`.
fn net_from_ord(ord: i32) -> FFINetwork {
    match ord {
        0 => FFINetwork::Mainnet,
        2 => FFINetwork::Devnet,
        3 => FFINetwork::Regtest,
        _ => FFINetwork::Testnet,
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
        let ffi_rows: Vec<IdentityPubkeyFFI> = decoded
            .iter()
            .map(|row| IdentityPubkeyFFI {
                key_id: row.key_id,
                key_type: row.key_type,
                purpose: row.purpose,
                security_level: row.security_level,
                pubkey_bytes: row.pubkey_bytes.as_ptr(),
                pubkey_len: row.pubkey_bytes.len(),
                read_only: row.read_only,
                contract_bounds_kind: row.contract_bounds_kind,
                contract_bounds_id: row
                    .contract_bounds_id
                    .as_ref()
                    .map_or(ptr::null(), |b| b.as_ptr()),
                contract_bounds_document_type: row
                    .contract_bounds_document_type
                    .as_ref()
                    .map_or(ptr::null(), |c| c.as_ptr()),
            })
            .collect();

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

/// One decoded add-key row with owned buffers backing the FFI pointers.
struct DecodedPubkeyRow {
    key_id: u32,
    key_type: u8,
    purpose: u8,
    security_level: u8,
    read_only: bool,
    contract_bounds_kind: u8,
    pubkey_bytes: Vec<u8>,
    contract_bounds_id: Option<[u8; 32]>,
    contract_bounds_document_type: Option<CString>,
}

/// Decode the identity-update add-keys BLOB documented on
/// [`Java_..._updateIdentity`]. Throws + returns None on a malformed blob.
fn decode_update_pubkeys_blob(env: &mut JNIEnv, arr: &JByteArray) -> Option<Vec<DecodedPubkeyRow>> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            // A null blob is a legitimate "no keys to add".
            let _ = env.exception_clear();
            return Some(Vec::new());
        }
    };
    if bytes.is_empty() {
        return Some(Vec::new());
    }

    let mut cursor = 0usize;
    let read = |cursor: &mut usize, n: usize| -> Option<Vec<u8>> {
        if *cursor + n > bytes.len() {
            return None;
        }
        let s = bytes[*cursor..*cursor + n].to_vec();
        *cursor += n;
        Some(s)
    };

    let Some(count_bytes) = read(&mut cursor, 4) else {
        throw_sdk_exception(env, 1, "addPubkeysBlob truncated (row count)");
        return None;
    };
    let count = u32::from_be_bytes(count_bytes.try_into().ok()?) as usize;
    // Length-before-allocation guard: each row is at least an 11-byte
    // fixed header, so a header claiming more rows than the remaining
    // payload can hold is malformed — prevents a huge `with_capacity`
    // abort from a raw-JNI blob.
    if count
        .checked_mul(11)
        .is_none_or(|need| bytes.len() - cursor < need)
    {
        throw_sdk_exception(
            env,
            1,
            &format!("addPubkeysBlob claims {count} rows but body is too short"),
        );
        return None;
    }
    let mut rows = Vec::with_capacity(count);
    for i in 0..count {
        let Some(fixed) = read(&mut cursor, 4 + 1 + 1 + 1 + 1 + 1 + 2) else {
            throw_sdk_exception(
                env,
                1,
                &format!("addPubkeysBlob truncated at row {i} header"),
            );
            return None;
        };
        let key_id = u32::from_be_bytes([fixed[0], fixed[1], fixed[2], fixed[3]]);
        // The Kotlin encoder writes this field with writeInt (signed); a
        // set sign bit means a negative key id crossed the boundary.
        if key_id > i32::MAX as u32 {
            throw_sdk_exception(
                env,
                1,
                &format!("addPubkeysBlob row {i} keyId must be non-negative"),
            );
            return None;
        }
        let key_type = fixed[4];
        let purpose = fixed[5];
        let security_level = fixed[6];
        let read_only = fixed[7] != 0;
        let contract_bounds_kind = fixed[8];
        let pubkey_len = u16::from_be_bytes([fixed[9], fixed[10]]) as usize;

        let Some(pubkey_bytes) = read(&mut cursor, pubkey_len) else {
            throw_sdk_exception(
                env,
                1,
                &format!("addPubkeysBlob truncated at row {i} pubkey"),
            );
            return None;
        };

        let mut contract_bounds_id: Option<[u8; 32]> = None;
        let mut contract_bounds_document_type: Option<CString> = None;
        if contract_bounds_kind != 0 {
            let Some(id_bytes) = read(&mut cursor, 32) else {
                throw_sdk_exception(
                    env,
                    1,
                    &format!("addPubkeysBlob truncated at row {i} contractBoundsId"),
                );
                return None;
            };
            let mut id = [0u8; 32];
            id.copy_from_slice(&id_bytes);
            contract_bounds_id = Some(id);

            if contract_bounds_kind == 2 {
                let Some(dt_len_bytes) = read(&mut cursor, 2) else {
                    throw_sdk_exception(
                        env,
                        1,
                        &format!("addPubkeysBlob truncated at row {i} docTypeLen"),
                    );
                    return None;
                };
                let dt_len = u16::from_be_bytes(dt_len_bytes.try_into().ok()?) as usize;
                let Some(dt_bytes) = read(&mut cursor, dt_len) else {
                    throw_sdk_exception(
                        env,
                        1,
                        &format!("addPubkeysBlob truncated at row {i} docType"),
                    );
                    return None;
                };
                match CString::new(dt_bytes) {
                    Ok(c) => contract_bounds_document_type = Some(c),
                    Err(_) => {
                        throw_sdk_exception(
                            env,
                            1,
                            &format!("addPubkeysBlob row {i} docType had an interior NUL"),
                        );
                        return None;
                    }
                }
            }
        }

        rows.push(DecodedPubkeyRow {
            key_id,
            key_type,
            purpose,
            security_level,
            read_only,
            contract_bounds_kind,
            pubkey_bytes,
            contract_bounds_id,
            contract_bounds_document_type,
        });
    }
    Some(rows)
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
/// `platform_wallet_create_encrypted_document_with_signer`.
///
/// The SDK derives the identity encryption key, seals `payload` into the
/// legacy `version ‖ IV ‖ AES-256-CBC` blob, and writes
/// `{keyIndex, encryptionKeyIndex, encryptedMetadata}`. `encryptionKeyIndex` is
/// the app's per-document index; `version` is the payload version byte
/// (`1` = protobuf); `payload` is the already-serialized opaque plaintext (a
/// protobuf `TxMetadataBatch`) — the SDK does not parse it. Returns the
/// confirmed document's canonical JSON (its 32-byte id is the base58 `$id`
/// field); null after throwing on error.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentCreateEncrypted(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
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
        if encryption_key_index < 0 {
            throw_sdk_exception(env, 1, "encryptionKeyIndex must be non-negative");
            return ptr::null_mut();
        }
        if !(0..=255).contains(&version) {
            throw_sdk_exception(env, 1, "version must be in 0..=255");
            return ptr::null_mut();
        }
        let payload_bytes = match env.convert_byte_array(&payload) {
            Ok(b) => b,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "payload byte[] was null/invalid");
                return ptr::null_mut();
            }
        };

        let mut out_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_create_encrypted_document_with_signer(
                wallet_handle as Handle,
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                encryption_key_index as u32,
                version as u8,
                payload_bytes.as_ptr(),
                payload_bytes.len(),
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
                "encrypted document create returned success but no canonical JSON",
            );
            return ptr::null_mut();
        }
        let json = unsafe { CStr::from_ptr(out_json) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::platform_wallet_string_free(out_json) };

        env.new_string(json)
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
/// `version == 1`). Documents that can't be decrypted are skipped Rust-side.
/// Null after throwing on error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_documentFetchEncrypted(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    contract_id: JByteArray,
    document_type: JString,
    since_ms: jlong,
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
        if since_ms < 0 {
            throw_sdk_exception(env, 1, "sinceMs must be non-negative");
            return ptr::null_mut();
        }

        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_fetch_encrypted_documents(
                wallet_handle as Handle,
                owner.as_ptr(),
                contract.as_ptr(),
                doc_type.as_ptr(),
                since_ms as u64,
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
                "encrypted document fetch returned success but no JSON",
            );
            return ptr::null_mut();
        }
        let json = unsafe { CStr::from_ptr(out_json) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::platform_wallet_string_free(out_json) };

        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
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
