//! JNI bridge for identity registration, discovery, key preview and
//! DPNS name registration on the platform-wallet `IdentityWallet`.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.IdentityNative`,
//! driven by `org.dashfoundation.dashsdk.identity.IdentityRegistration`.
//!
//! ## What lives here (and what deliberately doesn't)
//!
//! Every export is a thin marshaler over a SINGLE `platform-wallet-ffi`
//! entry point — no stitching of multiple Rust calls, per
//! `packages/kotlin-sdk/CLAUDE.md`. The wallet-balance-funded
//! registration (`platform_wallet_register_identity_with_funding_signer`)
//! is the one call the app's `RegistrationCoordinator` body invokes; the
//! asset-lock-resume, shielded-pool, and platform-address funding paths
//! are separate FFI entry points left for later milestones.
//!
//! ## Result convention
//!
//! These entry points return `PlatformWalletFFIResult` (platform-wallet's
//! own error enum), so errors go through the shared
//! [`crate::support::take_pwffi_error`] — the same mapping
//! `wallet_manager.rs` uses — rather than `rs-sdk-ffi`'s `DashSDKResult`
//! path in `results.rs`.
//!
//! ## Copy-before-return
//!
//! The key-preview and discovery entry points hand back heap arrays owned
//! by Rust; every trampoline copies the payload into JVM objects and then
//! calls the paired `*_free` (which also zeroizes the private-key material)
//! before returning, so no Rust allocation escapes the call.

#![allow(clippy::missing_safety_doc)]

use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jbyteArray, jint, jlong};
use jni::JNIEnv;
use platform_wallet_ffi::error::platform_wallet_ffi_result_free;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::identity_discovery::DiscoveredIdentityIdsFFI;
use platform_wallet_ffi::identity_key_preview::{IdentityKeyPreviewFFI, IdentityKeyPreviewsFFI};
use platform_wallet_ffi::identity_private_key_at_slot::IdentityPrivateKeyFFI;
use platform_wallet_ffi::identity_registration::IdentityFundingInputFFI;
use platform_wallet_ffi::identity_registration_with_signer::IdentityPubkeyFFI;
use platform_wallet_ffi::types::FFINetwork;
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

// ── Canonical registration key-role table ─────────────────────────────

/// DPP `KeyType::ECDSA_SECP256K1` discriminant byte.
const KEY_TYPE_ECDSA_SECP256K1: u8 = 0;
/// DPP `Purpose::AUTHENTICATION` discriminant byte.
const PURPOSE_AUTHENTICATION: u8 = 0;
/// DPP `Purpose::TRANSFER` discriminant byte.
const PURPOSE_TRANSFER: u8 = 3;
/// DPP `SecurityLevel::MASTER` discriminant byte.
const SECURITY_LEVEL_MASTER: u8 = 0;
/// DPP `SecurityLevel::CRITICAL` discriminant byte.
const SECURITY_LEVEL_CRITICAL: u8 = 1;
/// DPP `SecurityLevel::HIGH` discriminant byte.
const SECURITY_LEVEL_HIGH: u8 = 2;

/// Canonical `(key_type, purpose, security_level)` for the registration
/// key at `key_id`. This is the single Android source of truth for the
/// per-slot identity-key role layout, byte-for-byte identical to the iOS
/// reference (`packages/rs-platform-wallet-ffi/src/identity_derive_and_persist.rs`,
/// and `CreateIdentityView.defaultKeyCount` in the SwiftExampleApp):
///
/// | key_id | key_type        | purpose        | security_level |
/// |--------|-----------------|----------------|----------------|
/// | 0      | ECDSA_SECP256K1 | AUTHENTICATION | MASTER         |
/// | 1      | ECDSA_SECP256K1 | AUTHENTICATION | CRITICAL       |
/// | 2      | ECDSA_SECP256K1 | AUTHENTICATION | HIGH           |
/// | 3      | ECDSA_SECP256K1 | TRANSFER       | CRITICAL       |
/// | > 3    | ECDSA_SECP256K1 | AUTHENTICATION | HIGH           |
///
/// - keyId 0 (MASTER/AUTH) signs the IdentityCreate transition.
/// - keyId 1 (CRITICAL/AUTH) signs token state transitions —
///   `combined_security_level_requirement` collapses any batch with a
///   token transition to `[CRITICAL]`, so without it the identity can't
///   mint / burn / freeze tokens.
/// - keyId 2 (HIGH/AUTH) signs general document / DPNS / contract
///   transitions.
/// - keyId 3 (TRANSFER/CRITICAL) signs IdentityCreditTransfer /
///   IdentityCreditWithdrawal — without it those broadcasts are rejected
///   on-chain with "no transfer public key".
///
/// Previously this JNI hardcoded `purpose = AUTHENTICATION` for every row
/// and `security_level = MASTER if key_id == 0 else HIGH`, so a freshly
/// created identity had no CRITICAL auth key and no TRANSFER key, and all
/// token / credit-transfer / withdrawal writes failed validation right
/// after creation. If DPP renumbers any discriminant, update this table.
pub(crate) fn role_for_registration_key_id(key_id: u32) -> (u8, u8, u8) {
    let (purpose, security_level) = match key_id {
        0 => (PURPOSE_AUTHENTICATION, SECURITY_LEVEL_MASTER),
        1 => (PURPOSE_AUTHENTICATION, SECURITY_LEVEL_CRITICAL),
        2 => (PURPOSE_AUTHENTICATION, SECURITY_LEVEL_HIGH),
        3 => (PURPOSE_TRANSFER, SECURITY_LEVEL_CRITICAL),
        _ => (PURPOSE_AUTHENTICATION, SECURITY_LEVEL_HIGH),
    };
    (KEY_TYPE_ECDSA_SECP256K1, purpose, security_level)
}

/// Read a required 32-byte id from a Java `byte[]`; throws + returns None
/// on the wrong length or a JNI error.
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

// ── Key preview ───────────────────────────────────────────────────────

/// Derive the first `count` MASTER identity-authentication keypairs the
/// wallet would probe during a discovery scan, starting at
/// `startIndex` — a pure-compute view with no Platform RPCs.
///
/// Returns a flat `byte[]` BLOB the Kotlin side decodes into key-preview
/// rows (marshalling only — the policy / derivation lives entirely in
/// Rust). Layout (all integers big-endian):
///
/// ```text
/// u32  row_count
/// repeat row_count times:
///   u32  identity_index
///   u16  path_len
///   u8[path_len]  derivation_path (UTF-8)
///   u8[33]        compressed public key
///   u8[32]        raw private-key scalar
/// ```
///
/// The Rust preview buffer (including the sensitive private material) is
/// freed via `platform_wallet_preview_identity_registration_keys_free`
/// — which zeroizes the scalars — before this returns. `count < 0` uses
/// the Rust gap-limit default.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityNative_previewRegistrationKeys(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    resolver_handle: jlong,
    start_index: jint,
    count: jint,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut previews = IdentityKeyPreviewsFFI {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_preview_identity_registration_keys(
                wallet_handle as Handle,
                resolver_handle as *mut MnemonicResolverHandle,
                start_index.max(0) as u32,
                count,
                &mut previews as *mut IdentityKeyPreviewsFFI,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        let blob = unsafe { encode_preview_rows(&previews) };
        // Free (and zeroize) the Rust-owned preview buffer now that the
        // payload lives in `blob`.
        unsafe {
            platform_wallet_ffi::platform_wallet_preview_identity_registration_keys_free(
                &mut previews as *mut IdentityKeyPreviewsFFI,
            )
        };

        env.byte_array_from_slice(&blob)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Derive the full identity-registration key **set** for a single
/// identity: keyId 0..`count` at the fixed `identityIndex`. Unlike
/// [`Java_..._previewRegistrationKeys`] (which fixes the MASTER key slot
/// and walks the *identity* index for the discovery preview), this fixes
/// the identity index and walks the *key* index — so it returns every
/// keypair a freshly-created identity is built from (keyId 0..N).
///
/// `count < 0` derives the canonical default set
/// (`IDENTITY_REGISTRATION_KEY_SET_COUNT` = 4: MASTER auth, CRITICAL
/// auth, HIGH auth, TRANSFER/CRITICAL). The per-key DPP role is applied
/// positionally by keyId at registration time
/// (`role_for_registration_key_id`), NOT carried on the row — every row
/// is an ECDSA_SECP256K1 keypair.
///
/// Returns the same flat `byte[]` BLOB layout as
/// [`Java_..._previewRegistrationKeys`], decoded by
/// `IdentityKeyPreview.decodeAll`. The Rust buffer (incl. the sensitive
/// private material) is zeroized + freed before this returns.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityNative_previewRegistrationKeySet(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    resolver_handle: jlong,
    identity_index: jint,
    count: jint,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        if identity_index < 0 {
            throw_sdk_exception(env, 1, "identityIndex must be non-negative");
            return ptr::null_mut();
        }
        let mut previews = IdentityKeyPreviewsFFI {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_preview_identity_registration_key_set(
                wallet_handle as Handle,
                resolver_handle as *mut MnemonicResolverHandle,
                identity_index as u32,
                count,
                &mut previews as *mut IdentityKeyPreviewsFFI,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        let blob = unsafe { encode_preview_rows(&previews) };
        // Free (and zeroize) the Rust-owned preview buffer now that the
        // payload lives in `blob`. Same row layout as the discovery
        // preview, so the same `_free` reclaims it.
        unsafe {
            platform_wallet_ffi::platform_wallet_preview_identity_registration_keys_free(
                &mut previews as *mut IdentityKeyPreviewsFFI,
            )
        };

        env.byte_array_from_slice(&blob)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Serialize the preview rows into the BLOB layout documented on
/// [`Java_..._previewRegistrationKeys`]. Copies every field out of the
/// FFI buffer so the caller can free it immediately after.
///
/// # Safety
/// `previews` must be a populated `IdentityKeyPreviewsFFI` with `count`
/// valid rows (or an empty struct).
unsafe fn encode_preview_rows(previews: &IdentityKeyPreviewsFFI) -> Vec<u8> {
    let mut out = Vec::new();
    if previews.items.is_null() || previews.count == 0 {
        out.extend_from_slice(&0u32.to_be_bytes());
        return out;
    }
    let rows = std::slice::from_raw_parts(previews.items, previews.count);
    out.extend_from_slice(&(rows.len() as u32).to_be_bytes());
    for row in rows {
        out.extend_from_slice(&row.identity_index.to_be_bytes());

        let path = if row.derivation_path.is_null() {
            Vec::new()
        } else {
            CStr::from_ptr(row.derivation_path).to_bytes().to_vec()
        };
        // Paths are short (`m/9'/…`); a u16 length is ample.
        let path_len = path.len().min(u16::MAX as usize) as u16;
        out.extend_from_slice(&path_len.to_be_bytes());
        out.extend_from_slice(&path[..path_len as usize]);

        // Public key: always 33 bytes; copy exactly what the row reports.
        let pub_len = row.public_key_len.min(33);
        let mut pubkey = [0u8; 33];
        if !row.public_key.is_null() && pub_len > 0 {
            let src = std::slice::from_raw_parts(row.public_key, pub_len);
            pubkey[..pub_len].copy_from_slice(src);
        }
        out.extend_from_slice(&pubkey);

        // Raw private-key scalar (inline in the row).
        out.extend_from_slice(&row.private_key_bytes);
    }
    out
}

// ── Single-slot private-key derivation ────────────────────────────────

/// Derive the ready-to-persist 32-byte ECDSA private-key scalar for the
/// identity key at `(identityIndex, keyIndex)` on the wallet behind
/// `walletHandle`, and return it as a JVM `byte[32]`.
///
/// This is the JNI bridge over
/// `platform_wallet_derive_identity_private_key_at_slot` — the single
/// Rust entry point that performs the whole `mnemonic → seed → path →
/// key` derivation on the Rust side (the CLAUDE.md "one allowed
/// exception" shape). The Kotlin persistence handler then just encrypts
/// the returned bytes into Keystore-backed storage; it never derives.
///
/// The derivation source (resident wallet vs. resolver-provided mnemonic)
/// is chosen by the wallet's capability; `resolverHandle` is consulted
/// only for external-signable / watch-only wallets and may be `0` (null)
/// otherwise. The network + path shape are read from the wallet handle,
/// so Kotlin decides nothing.
///
/// The Rust-owned buffer (including the sensitive scalar) is zeroized and
/// freed via `platform_wallet_derive_identity_private_key_at_slot_free`
/// before this returns — the only copy that escapes is the JVM `byte[]`,
/// which the Kotlin caller is expected to scrub after storing.
///
/// Returns the 32-byte scalar on success, or `null` (with a
/// `DashSDKException` thrown) on any derivation / marshalling error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityNative_deriveIdentityPrivateKey(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    resolver_handle: jlong,
    identity_index: jint,
    key_index: jint,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        // Reject negative slot indices at the boundary — a clamped 0 would
        // silently derive (and persist) the wrong slot's key material.
        if identity_index < 0 {
            throw_sdk_exception(env, 1, "identityIndex must be non-negative");
            return ptr::null_mut();
        }
        if key_index < 0 {
            throw_sdk_exception(env, 1, "keyIndex must be non-negative");
            return ptr::null_mut();
        }
        let mut out_key = IdentityPrivateKeyFFI::empty();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_derive_identity_private_key_at_slot(
                wallet_handle as Handle,
                resolver_handle as *mut MnemonicResolverHandle,
                identity_index as u32,
                key_index as u32,
                &mut out_key as *mut IdentityPrivateKeyFFI,
            )
        };
        if take_pwffi_error(env, result) {
            // Free even on the error path — the FFI pre-clears to empty,
            // so this is a safe no-op, but keep the pairing explicit.
            unsafe {
                platform_wallet_ffi::platform_wallet_derive_identity_private_key_at_slot_free(
                    &mut out_key as *mut IdentityPrivateKeyFFI,
                )
            };
            return ptr::null_mut();
        }

        // Build the JVM byte[] straight from the Rust-owned buffer BEFORE
        // freeing it — no independent stack copy of the scalar is left
        // behind for the free's zeroize pass to miss.
        let jarr = env
            .byte_array_from_slice(&out_key.private_key_bytes)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut());

        // Zeroize + free the Rust-owned buffer (scrubs the scalar and
        // reclaims the path string).
        unsafe {
            platform_wallet_ffi::platform_wallet_derive_identity_private_key_at_slot_free(
                &mut out_key as *mut IdentityPrivateKeyFFI,
            )
        };

        jarr
    })
}

/// FFINetwork ordinal → the crate's `FFINetwork` enum
/// (0=Mainnet, 2=Devnet, 3=Regtest, else Testnet). Kept in step with
/// `persistence::net_from_ord`.
fn net_from_ord(ord: i32) -> FFINetwork {
    match ord {
        0 => FFINetwork::Mainnet,
        2 => FFINetwork::Devnet,
        3 => FFINetwork::Regtest,
        _ => FFINetwork::Testnet,
    }
}

/// Resolver-keyed sibling of [`Java_..._deriveIdentityPrivateKey`] for the
/// **persistence-callback** path.
///
/// The identity-key persistence callback fires synchronously from inside
/// a platform-wallet operation that holds the wallet-manager **write**
/// lock (`registration.rs` persists the identity changeset under
/// `wallet_manager.write().await`). Any derive that re-locks the wallet
/// manager — including the handle-keyed
/// `platform_wallet_derive_identity_private_key_at_slot`, whose
/// capability check does a `blocking_read` — would deadlock on that same
/// RwLock. This variant routes through
/// `dash_sdk_derive_identity_key_at_slot_with_resolver`, which is
/// **pure** (resolver → mnemonic → master → derive) and never touches the
/// wallet-manager registry, so it is safe to call from the callback.
///
/// The network + `walletId` are passed explicitly because the callback
/// has no wallet handle. The resolver resolves the mnemonic keyed by
/// `walletId`. Returns the 32-byte scalar; the Rust row (incl. WIF +
/// scalar) is zeroized + freed before return.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityNative_deriveIdentityPrivateKeyWithResolver(
    mut env: JNIEnv,
    _class: JClass,
    network_ord: jint,
    wallet_id: JByteArray,
    resolver_handle: jlong,
    identity_index: jint,
    key_index: jint,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        // Reject negative slot indices at the boundary — a clamped 0 would
        // silently derive (and persist) the wrong slot's key material.
        if identity_index < 0 {
            throw_sdk_exception(env, 1, "identityIndex must be non-negative");
            return ptr::null_mut();
        }
        if key_index < 0 {
            throw_sdk_exception(env, 1, "keyIndex must be non-negative");
            return ptr::null_mut();
        }
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };

        let mut out_row = IdentityKeyPreviewFFI::empty();
        let result = unsafe {
            platform_wallet_ffi::dash_sdk_derive_identity_key_at_slot_with_resolver(
                net_from_ord(network_ord),
                wid.as_ptr(),
                resolver_handle as *mut MnemonicResolverHandle,
                identity_index as u32,
                key_index as u32,
                &mut out_row as *mut IdentityKeyPreviewFFI,
            )
        };
        if take_pwffi_error(env, result) {
            unsafe {
                platform_wallet_ffi::dash_sdk_derive_identity_key_at_slot_free(
                    &mut out_row as *mut IdentityKeyPreviewFFI,
                )
            };
            return ptr::null_mut();
        }

        // Build the JVM byte[] straight from the Rust-owned row BEFORE
        // freeing it — no independent stack copy of the scalar is left
        // behind for the free's zeroize pass to miss.
        let jarr = env
            .byte_array_from_slice(&out_row.private_key_bytes)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut());

        unsafe {
            platform_wallet_ffi::dash_sdk_derive_identity_key_at_slot_free(
                &mut out_row as *mut IdentityKeyPreviewFFI,
            )
        };

        jarr
    })
}

/// Keypair variant of the resolver-keyed slot derive: returns
/// `[privateKey: byte[32], publicKey: byte[]]` from the same
/// `IdentityKeyPreviewFFI` row the scalar-only export reads (the public
/// half was previously discarded — it is required by
/// `IdentityUpdateTransition` add-key rows). Lock-free like its sibling:
/// safe from persistence-callback context.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityNative_deriveIdentityKeyPairWithResolver(
    mut env: JNIEnv,
    _class: JClass,
    network_ord: jint,
    wallet_id: JByteArray,
    resolver_handle: jlong,
    identity_index: jint,
    key_index: jint,
) -> jni::sys::jobjectArray {
    guard(&mut env, ptr::null_mut(), |env| {
        // Reject negative slot indices at the boundary — a clamped 0 would
        // silently derive (and persist) the wrong slot's key material.
        if identity_index < 0 {
            throw_sdk_exception(env, 1, "identityIndex must be non-negative");
            return ptr::null_mut();
        }
        if key_index < 0 {
            throw_sdk_exception(env, 1, "keyIndex must be non-negative");
            return ptr::null_mut();
        }
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };

        let mut out_row = IdentityKeyPreviewFFI::empty();
        let result = unsafe {
            platform_wallet_ffi::dash_sdk_derive_identity_key_at_slot_with_resolver(
                net_from_ord(network_ord),
                wid.as_ptr(),
                resolver_handle as *mut MnemonicResolverHandle,
                identity_index as u32,
                key_index as u32,
                &mut out_row as *mut IdentityKeyPreviewFFI,
            )
        };
        if take_pwffi_error(env, result) {
            unsafe {
                platform_wallet_ffi::dash_sdk_derive_identity_key_at_slot_free(
                    &mut out_row as *mut IdentityKeyPreviewFFI,
                )
            };
            return ptr::null_mut();
        }

        // Copy BOTH halves before the free (callback-window rule). The
        // private half is wrapped in `Zeroizing` so the stack copy is
        // scrubbed on drop (the pubkey copy is public — plain Vec is fine).
        let scalar = zeroize::Zeroizing::new(out_row.private_key_bytes);
        let pubkey: Vec<u8> = if out_row.public_key.is_null() || out_row.public_key_len == 0 {
            Vec::new()
        } else {
            unsafe {
                std::slice::from_raw_parts(out_row.public_key, out_row.public_key_len).to_vec()
            }
        };
        unsafe {
            platform_wallet_ffi::dash_sdk_derive_identity_key_at_slot_free(
                &mut out_row as *mut IdentityKeyPreviewFFI,
            )
        };
        if pubkey.is_empty() {
            crate::support::throw_sdk_exception(
                env,
                99,
                "slot derive returned no public key bytes",
            );
            return ptr::null_mut();
        }

        let build = (|| -> Result<jni::sys::jobjectArray, jni::errors::Error> {
            let priv_arr = env.byte_array_from_slice(&*scalar)?;
            let pub_arr = env.byte_array_from_slice(&pubkey)?;
            let byte_array_class = env.find_class("[B")?;
            let result =
                env.new_object_array(2, byte_array_class, jni::objects::JObject::null())?;
            env.set_object_array_element(&result, 0, priv_arr)?;
            env.set_object_array_element(&result, 1, pub_arr)?;
            Ok(result.into_raw())
        })();
        build.unwrap_or_else(|_| {
            let _ = env.exception_clear();
            crate::support::throw_sdk_exception(env, 99, "keypair marshalling failed");
            ptr::null_mut()
        })
    })
}

// ── Registration (wallet-balance funded) ──────────────────────────────

/// Register a new identity funded from the wallet's Core balance, driven
/// by an external identity signer plus a mnemonic resolver for the
/// asset-lock's credit-spend signature.
///
/// This is the single FFI entry point the app's `RegistrationCoordinator`
/// invokes — no orchestration on the Kotlin side. The caller (Kotlin) has
/// already derived + persisted the identity keys via
/// [`Java_..._previewRegistrationKeys`], so `pubkeysBlob` is the same flat
/// layout `previewRegistrationKeys` produced, trimmed to the keys being
/// registered (row `identity_index` is ignored here — the pubkey rows are
/// read positionally as `keyId = index`).
///
/// `pubkeysBlob` layout (big-endian):
/// ```text
/// u32 row_count
/// repeat: u32 keyId, u16 pubkey_len, u8[pubkey_len] compressed pubkey
/// ```
/// Every key is registered as an ECDSA_SECP256K1 / AUTHENTICATION key at
/// the security level implied by its position (row 0 = MASTER). The Rust
/// side validates the security-level layout.
///
/// Returns the 32-byte identity id as a `byte[]`. The `ManagedIdentity`
/// handle the FFI produces is destroyed here — Room learns of the new
/// identity through the persistence changeset, not through this handle.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityNative_registerIdentityWithFunding(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    amount_duffs: jlong,
    account_index: jint,
    identity_index: jint,
    pubkeys_blob: JByteArray,
    signer_handle: jlong,
    core_signer_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        // Reject sign errors at the boundary — a negative amount / index
        // would otherwise bit-cast to a huge unsigned value (and a clamped
        // 0 amount would post a meaningless 0-duff registration).
        if amount_duffs <= 0 {
            throw_sdk_exception(env, 1, "amountDuffs must be positive");
            return ptr::null_mut();
        }
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }
        if identity_index < 0 {
            throw_sdk_exception(env, 1, "identityIndex must be non-negative");
            return ptr::null_mut();
        }

        // Decode the pubkey rows into owned buffers that outlive the FFI
        // call (the FFI borrows `pubkey_bytes` for the call duration).
        let Some(decoded) = decode_pubkeys_blob(env, &pubkeys_blob) else {
            return ptr::null_mut();
        };
        if decoded.is_empty() {
            throw_sdk_exception(env, 1, "pubkeysBlob contained no keys");
            return ptr::null_mut();
        }

        // Build the FFI rows referencing the owned buffers. `read_only`
        // false, no contract bounds (auth / transfer keys never carry
        // bounds — only ENCRYPTION / DECRYPTION do). The per-key role
        // (key_type / purpose / security_level) is the canonical,
        // positional function of `key_id` (see
        // `role_for_registration_key_id`), matching iOS exactly — so a
        // freshly created identity gets keyId 0 MASTER/AUTH, keyId 1
        // CRITICAL/AUTH, keyId 2 HIGH/AUTH, keyId 3 TRANSFER/CRITICAL.
        let ffi_rows: Vec<IdentityPubkeyFFI> = decoded
            .iter()
            .map(|(key_id, bytes)| {
                let (key_type, purpose, security_level) = role_for_registration_key_id(*key_id);
                IdentityPubkeyFFI {
                    key_id: *key_id,
                    key_type,
                    purpose,
                    security_level,
                    pubkey_bytes: bytes.as_ptr(),
                    pubkey_len: bytes.len(),
                    read_only: false,
                    contract_bounds_kind: 0,
                    contract_bounds_id: ptr::null(),
                    contract_bounds_document_type: ptr::null(),
                }
            })
            .collect();

        let mut out_id = [0u8; 32];
        let mut out_managed: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_register_identity_with_funding_signer(
                wallet_handle as Handle,
                amount_duffs as u64,
                account_index as u32,
                identity_index as u32,
                ffi_rows.as_ptr(),
                ffi_rows.len(),
                signer_handle as *mut SignerHandle,
                core_signer_handle as *mut MnemonicResolverHandle,
                &mut out_id as *mut [u8; 32],
                &mut out_managed as *mut Handle,
            )
        };
        // `decoded` / `ffi_rows` own the pubkey buffers the pointers
        // referenced; they stay in scope through the FFI call above.
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        // The new identity is folded into Rust's IdentityManager and lands
        // in Room via the persister changeset; the standalone managed
        // handle would otherwise leak, so drop it.
        if out_managed != 0 {
            let mut destroy = unsafe { platform_wallet_ffi::managed_identity_destroy(out_managed) };
            unsafe { platform_wallet_ffi_result_free(&mut destroy) };
        }

        env.byte_array_from_slice(&out_id)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Registration (Platform-address funded) ────────────────────────────

/// Register a new identity funded by the wallet's already-committed
/// Platform-payment (DIP-17) address balances — the ID-08 create path,
/// distinct from `registerIdentityWithFunding` (ID-01) which builds a new
/// Core asset lock. No Core-chain transaction is broadcast; the inputs are
/// existing Platform credits.
///
/// `pubkeysBlob` is the same flat layout `registerIdentityWithFunding`
/// consumes (`u32 rowCount` then per row `u32 keyId, u16 pubkeyLen,
/// pubkey`), each key stamped with its canonical DPP role by `keyId`.
///
/// `inputsBlob` is the funding-address row shape shared with the top-up /
/// transfer-to-addresses exports (`u32 rowCount` then per row
/// `u8 addressType (0 P2PKH / 1 P2SH), u8[20] hash, u64 credits`). The
/// on-chain nonces are auto-fetched Rust-side right before submit.
///
/// `signerHandle` is used for **both** signing roles — the new identity's
/// state-transition keys and the input Platform addresses. The underlying
/// `VTableSigner` dispatches by key-type byte, so one handle serves both
/// (the platform-address signing path is the same one ID-06 top-up drives).
///
/// Returns the 32-byte identity id. The standalone `ManagedIdentity` handle
/// the FFI produces is destroyed here — Room learns of the new identity
/// through the persistence changeset, not through this handle.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityNative_registerIdentityFromAddresses(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_index: jint,
    pubkeys_blob: JByteArray,
    signer_handle: jlong,
    inputs_blob: JByteArray,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        if identity_index < 0 {
            throw_sdk_exception(env, 1, "identityIndex must be non-negative");
            return ptr::null_mut();
        }

        let Some(decoded) = decode_pubkeys_blob(env, &pubkeys_blob) else {
            return ptr::null_mut();
        };
        if decoded.is_empty() {
            throw_sdk_exception(env, 1, "pubkeysBlob contained no keys");
            return ptr::null_mut();
        }

        let Some(input_rows) = crate::credits::decode_credit_rows(env, &inputs_blob, "inputsBlob")
        else {
            return ptr::null_mut();
        };
        if input_rows.is_empty() {
            throw_sdk_exception(env, 1, "inputsBlob contained no funding inputs");
            return ptr::null_mut();
        }

        // Same positional keyId → DPP role assignment as ID-01 (keyId 0
        // MASTER/AUTH, 1 CRITICAL/AUTH, 2 HIGH/AUTH, 3 TRANSFER/CRITICAL).
        let ffi_rows: Vec<IdentityPubkeyFFI> = decoded
            .iter()
            .map(|(key_id, bytes)| {
                let (key_type, purpose, security_level) = role_for_registration_key_id(*key_id);
                IdentityPubkeyFFI {
                    key_id: *key_id,
                    key_type,
                    purpose,
                    security_level,
                    pubkey_bytes: bytes.as_ptr(),
                    pubkey_len: bytes.len(),
                    read_only: false,
                    contract_bounds_kind: 0,
                    contract_bounds_id: ptr::null(),
                    contract_bounds_document_type: ptr::null(),
                }
            })
            .collect();

        let input_ffi: Vec<IdentityFundingInputFFI> = input_rows
            .into_iter()
            .map(|(address_type, hash, credits)| IdentityFundingInputFFI {
                address_type,
                hash,
                credits,
            })
            .collect();

        let mut out_id = [0u8; 32];
        let mut out_managed: Handle = 0;
        let result = unsafe {
            // Same signer handle for both the identity-key and
            // platform-address signing roles — the VTableSigner dispatches
            // by key-type byte (see the FFI's two-signer doc comment).
            platform_wallet_ffi::platform_wallet_register_identity_with_signer(
                wallet_handle as Handle,
                identity_index as u32,
                ffi_rows.as_ptr(),
                ffi_rows.len(),
                signer_handle as *mut SignerHandle,
                signer_handle as *mut SignerHandle,
                input_ffi.as_ptr(),
                input_ffi.len(),
                ptr::null(),
                &mut out_id as *mut [u8; 32],
                &mut out_managed as *mut Handle,
            )
        };
        // `decoded` / `ffi_rows` / `input_ffi` own the buffers the pointers
        // reference; they stay in scope through the FFI call above.
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        if out_managed != 0 {
            let mut destroy = unsafe { platform_wallet_ffi::managed_identity_destroy(out_managed) };
            unsafe { platform_wallet_ffi_result_free(&mut destroy) };
        }

        env.byte_array_from_slice(&out_id)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Decode the registration pubkeys BLOB into `(keyId, bytes)` rows whose
/// buffers the caller keeps alive across the FFI call. Throws + returns
/// None on a malformed blob.
pub(crate) fn decode_pubkeys_blob(
    env: &mut JNIEnv,
    arr: &JByteArray,
) -> Option<Vec<(u32, Vec<u8>)>> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "pubkeysBlob was null/invalid");
            return None;
        }
    };
    let mut cursor = 0usize;
    let read = |cursor: &mut usize, n: usize| -> Option<&[u8]> {
        if *cursor + n > bytes.len() {
            return None;
        }
        let s = &bytes[*cursor..*cursor + n];
        *cursor += n;
        Some(s)
    };
    let Some(count_bytes) = read(&mut cursor, 4) else {
        throw_sdk_exception(env, 1, "pubkeysBlob truncated (row count)");
        return None;
    };
    let count = u32::from_be_bytes(count_bytes.try_into().ok()?) as usize;
    // Length-before-allocation guard: each row is at least 6 bytes
    // (u32 keyId + u16 len), so a header claiming more rows than the
    // remaining payload can hold is malformed — prevents a huge
    // `with_capacity` abort from a raw-JNI blob.
    if count
        .checked_mul(6)
        .is_none_or(|need| bytes.len() - cursor < need)
    {
        throw_sdk_exception(
            env,
            1,
            &format!("pubkeysBlob claims {count} rows but body is too short"),
        );
        return None;
    }
    let mut rows = Vec::with_capacity(count);
    for i in 0..count {
        let Some(id_bytes) = read(&mut cursor, 4) else {
            throw_sdk_exception(env, 1, &format!("pubkeysBlob truncated at row {i} keyId"));
            return None;
        };
        let key_id = u32::from_be_bytes(id_bytes.try_into().ok()?);
        // The Kotlin encoder writes this field with writeInt (signed); a
        // set sign bit means a negative key id crossed the boundary.
        if key_id > i32::MAX as u32 {
            throw_sdk_exception(
                env,
                1,
                &format!("pubkeysBlob row {i} keyId must be non-negative"),
            );
            return None;
        }
        let Some(len_bytes) = read(&mut cursor, 2) else {
            throw_sdk_exception(env, 1, &format!("pubkeysBlob truncated at row {i} len"));
            return None;
        };
        let len = u16::from_be_bytes(len_bytes.try_into().ok()?) as usize;
        let Some(pubkey) = read(&mut cursor, len) else {
            throw_sdk_exception(env, 1, &format!("pubkeysBlob truncated at row {i} pubkey"));
            return None;
        };
        rows.push((key_id, pubkey.to_vec()));
    }
    Some(rows)
}

// ── Discovery ─────────────────────────────────────────────────────────

/// Scan the wallet's identity-authentication tree for registered
/// identities (gap-limit walk on the Rust side). Returns a `byte[]`
/// holding the concatenated 32-byte identity ids (length is a multiple of
/// 32); the Kotlin side splits them. `startIndex < 0` uses the Rust
/// default start; `gapLimit` bounds the consecutive-miss window.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityNative_discoverIdentities(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    resolver_handle: jlong,
    start_index: jint,
    gap_limit: jint,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut found = DiscoveredIdentityIdsFFI {
            ids: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_discover_identities(
                wallet_handle as Handle,
                resolver_handle as *mut MnemonicResolverHandle,
                start_index as i64,
                gap_limit.max(0) as u32,
                &mut found as *mut DiscoveredIdentityIdsFFI,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        // Copy the ids out before freeing the Rust buffer.
        let mut flat = Vec::with_capacity(found.count * 32);
        if !found.ids.is_null() && found.count > 0 {
            let ids = unsafe { std::slice::from_raw_parts(found.ids, found.count) };
            for id in ids {
                flat.extend_from_slice(id);
            }
        }
        unsafe {
            platform_wallet_ffi::platform_wallet_discover_identities_free(
                &mut found as *mut DiscoveredIdentityIdsFFI,
            )
        };

        env.byte_array_from_slice(&flat)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── DPNS name registration ────────────────────────────────────────────

/// Register a DPNS name for an identity, signed via the external signer.
/// Works on watch-only wallets (no seed Rust-side). Returns the full
/// domain name (e.g. `"alice.dash"`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityNative_registerDpnsName(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    label: JString,
    signer_handle: jlong,
) -> jni::sys::jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return ptr::null_mut();
        };
        let label_str: String = match env.get_string(&label) {
            Ok(s) => s.into(),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "label string was null/invalid");
                return ptr::null_mut();
            }
        };
        let c_label = match CString::new(label_str) {
            Ok(c) => c,
            Err(_) => {
                throw_sdk_exception(env, 1, "label contained an interior NUL");
                return ptr::null_mut();
            }
        };

        let mut out_full: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_register_dpns_name_with_signer(
                wallet_handle as Handle,
                id.as_ptr(),
                c_label.as_ptr(),
                signer_handle as *mut SignerHandle,
                &mut out_full as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        if out_full.is_null() {
            throw_sdk_exception(env, 99, "DPNS register returned success but no domain name");
            return ptr::null_mut();
        }
        // Copy the name out, then free the Rust string.
        let full = unsafe { CStr::from_ptr(out_full) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::platform_wallet_string_free(out_full) };

        env.new_string(full)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Data-contract create ──────────────────────────────────────────────

/// Read an optional JVM string into an owned `CString`. `None` (a null
/// `JString` or an empty string) marshals to a null `*const c_char` so
/// the FFI treats the section as omitted. Returns `Err(())` (after
/// throwing) on a JNI read error or an interior-NUL string.
fn read_optional_cstring(
    env: &mut JNIEnv,
    s: &JString,
    field: &str,
) -> Result<Option<CString>, ()> {
    if s.is_null() {
        return Ok(None);
    }
    let raw: String = match env.get_string(s) {
        Ok(js) => js.into(),
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} string was invalid"));
            return Err(());
        }
    };
    if raw.is_empty() {
        return Ok(None);
    }
    match CString::new(raw) {
        Ok(c) => Ok(Some(c)),
        Err(_) => {
            throw_sdk_exception(env, 1, &format!("{field} contained an interior NUL"));
            Err(())
        }
    }
}

/// Create + broadcast a new data contract owned by `ownerIdentityId`,
/// signed via the external signer. Thin marshaler over
/// `platform_wallet_create_data_contract_with_signer` — the whole
/// build/validate/broadcast pipeline lives in platform-wallet.
///
/// `documentsSchemaJson` is required; `tokens`/`groups`/`keywords`/
/// `description`/`config` are optional (null or empty ⇒ omitted).
/// Returns the 32-byte created contract id.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityNative_createDataContract(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_identity_id: JByteArray,
    documents_schema_json: JString,
    tokens_schema_json: JString,
    groups_schema_json: JString,
    keywords_json: JString,
    description: JString,
    config_json: JString,
    signer_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_id32(env, &owner_identity_id, "ownerIdentityId") else {
            return ptr::null_mut();
        };

        // Documents schema is required.
        if documents_schema_json.is_null() {
            throw_sdk_exception(env, 1, "documentsSchemaJson is required");
            return ptr::null_mut();
        }
        let documents: String = match env.get_string(&documents_schema_json) {
            Ok(s) => s.into(),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "documentsSchemaJson was invalid");
                return ptr::null_mut();
            }
        };
        let Ok(documents_c) = CString::new(documents) else {
            throw_sdk_exception(env, 1, "documentsSchemaJson contained an interior NUL");
            return ptr::null_mut();
        };

        let (tokens, groups, keywords, desc, config) = {
            let Ok(t) = read_optional_cstring(env, &tokens_schema_json, "tokensSchemaJson") else {
                return ptr::null_mut();
            };
            let Ok(g) = read_optional_cstring(env, &groups_schema_json, "groupsSchemaJson") else {
                return ptr::null_mut();
            };
            let Ok(k) = read_optional_cstring(env, &keywords_json, "keywordsJson") else {
                return ptr::null_mut();
            };
            let Ok(d) = read_optional_cstring(env, &description, "description") else {
                return ptr::null_mut();
            };
            let Ok(c) = read_optional_cstring(env, &config_json, "configJson") else {
                return ptr::null_mut();
            };
            (t, g, k, d, c)
        };

        let opt_ptr = |c: &Option<CString>| -> *const c_char {
            c.as_ref().map_or(ptr::null(), |s| s.as_ptr())
        };

        let mut out_contract_id = [0u8; 32];
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_create_data_contract_with_signer(
                wallet_handle as Handle,
                owner.as_ptr(),
                documents_c.as_ptr(),
                opt_ptr(&tokens),
                opt_ptr(&groups),
                opt_ptr(&keywords),
                opt_ptr(&desc),
                opt_ptr(&config),
                signer_handle as *mut SignerHandle,
                out_contract_id.as_mut_ptr(),
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        env.byte_array_from_slice(&out_contract_id)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}
