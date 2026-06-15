//! Single-FFI identity-key derivation + persistence pipeline.
//!
//! Companion to [`crate::derive_and_persist_callbacks`] (which
//! defines the resolver / persister vtables) and the lower-level
//! [`crate::dash_sdk_derive_identity_keys_from_mnemonic`] (which
//! returns the derived keys to the caller and lets the caller
//! orchestrate persistence).
//!
//! # Why this entry point exists
//!
//! `swift-sdk/CLAUDE.md` forbids the Swift SDK from orchestrating
//! the BIP-39 → seed → master xpriv → derived xpriv → 32-byte
//! private key → Keychain pipeline. The lower-level FFI is enough
//! for Swift to *call* per-key derivation, but Swift then has to
//! pull the mnemonic out of Keychain itself, hand it across the
//! FFI as a C-string, and walk the returned rows writing each to
//! Keychain — i.e. the rule violation the rule was written for.
//!
//! This entry point pushes the loop down: Swift hands the Rust side
//! two callback handles (one resolver, one persister) and the Rust
//! side runs the entire pipeline, calling back into Swift only for
//! the iOS-only Keychain primitives. The Swift caller's
//! responsibility shrinks to:
//!
//! 1. Implement the resolver: given a `wallet_id`, copy the
//!    BIP-39 mnemonic into a Rust-owned buffer.
//! 2. Implement the persister: given a `(walletId, identityIndex,
//!    keyId, path, pubkey, pubkeyHash, privKey, keyType, purpose,
//!    securityLevel)` tuple, write a Keychain row.
//!
//! The mnemonic still crosses the FFI as a C-string (Keychain is
//! iOS-only, so Rust can't read it directly) but it now flows IN
//! to a Rust-owned `Zeroizing` buffer that is scrubbed at function
//! exit. The 32-byte ECDSA scalars never flow OUT — they stay on
//! the Rust stack until the persister callback writes them, and
//! are zeroized as soon as the loop iteration completes.
//!
//! # Policy lives in Rust
//!
//! Per-key metadata (key_type / purpose / security_level) is
//! computed on the Rust side using the DPP discriminants directly,
//! removing the policy-encoded-in-Swift block that the lower-
//! level FFI required the Swift caller to maintain. For the
//! identity-registration flow:
//!
//! | key_id | key_type        | purpose        | security_level |
//! |--------|-----------------|----------------|----------------|
//! | 0      | ECDSA_SECP256K1 | AUTHENTICATION | MASTER         |
//! | 1      | ECDSA_SECP256K1 | AUTHENTICATION | CRITICAL       |
//! | 2      | ECDSA_SECP256K1 | AUTHENTICATION | HIGH           |
//! | 3      | ECDSA_SECP256K1 | TRANSFER       | CRITICAL       |
//! | > 3    | ECDSA_SECP256K1 | AUTHENTICATION | HIGH           |
//!
//! Mirrors rs-dpp's canonical
//! `main_keys_with_random_authentication_keys_*` layout (MASTER →
//! CRITICAL → HIGH). The CRITICAL key at id 1 is what signs token
//! state transitions — rs-dpp's
//! `combined_security_level_requirement` collapses any batch
//! containing a token transition to `[CRITICAL]` only, so without
//! a CRITICAL key the identity cannot mint / burn / freeze tokens.
//!
//! Key id 3 is a `TRANSFER`/`CRITICAL` key. Dash Platform requires a
//! `purpose=TRANSFER`, `security_level=CRITICAL`, ECDSA_SECP256K1 key
//! to sign `IdentityCreditTransfer` (ID-04) and
//! `IdentityCreditWithdrawal` (ID-10) state transitions; without it
//! those broadcasts are rejected on-chain with
//! `Protocol error: missing key: no transfer public key`. So the
//! canonical default set provisions one. Slots 0/1/2 keep their
//! pre-existing `(purpose, security_level)` bytes byte-for-byte, so a
//! caller that still requests `key_count == 3` is completely
//! unchanged. `TRANSFER` keys take no contract bounds (only
//! ENCRYPTION/DECRYPTION require them), and this function never sets
//! bounds anyway.
//!
//! If DPP renumbers any of those enum discriminants, this file is
//! the single place that needs updating — the Swift caller just
//! writes whatever bytes it's told.
//!
//! # Output shape
//!
//! Returns a heap-allocated [`IdentityRegistrationKeyDerivationsFFI`]
//! containing pubkeys + paths only — `private_key_bytes` is left
//! at the zero-initialised default (`[0u8; 32]`) and
//! `private_key_wif` is null. The `_free` function on the existing
//! struct handles the zero/null case correctly. The returned rows
//! exist so the Swift caller can build its `IdentityPubkey` rows
//! for the upcoming `registerIdentityFromAddresses(...signer:)`
//! call without performing a second derivation pass.

use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::types::{FFINetwork, Network};
use dashcore::hashes::Hash;
use dashcore::secp256k1::Secp256k1;
use key_wallet::bip32::{ExtendedPrivKey, ExtendedPubKey};
use zeroize::{Zeroize, Zeroizing};

use crate::derive_and_persist_callbacks::{
    IdentityKeyPersisterHandle, PersistKeyArgs, PERSIST_KEY_SUCCESS,
};
use crate::error::*;
use crate::identity_key_preview::IdentityKeyPreviewFFI;
use crate::identity_keys_from_mnemonic::{
    identity_auth_derivation_path, parse_mnemonic_any_language,
};
use crate::identity_registration_with_signer::IdentityRegistrationKeyDerivationsFFI;
use crate::{check_ptr, unwrap_result_or_return};
use rs_sdk_ffi::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};

/// DPP `KeyType::ECDSA_SECP256K1` discriminant byte.
const KEY_TYPE_ECDSA_SECP256K1: u8 = 0;
/// DPP `Purpose::AUTHENTICATION` discriminant byte.
const PURPOSE_AUTHENTICATION: u8 = 0;
/// DPP `Purpose::TRANSFER` discriminant byte. Required (with
/// `SECURITY_LEVEL_CRITICAL`) to sign `IdentityCreditTransfer` /
/// `IdentityCreditWithdrawal` state transitions.
const PURPOSE_TRANSFER: u8 = 3;
/// DPP `SecurityLevel::MASTER` discriminant byte.
const SECURITY_LEVEL_MASTER: u8 = 0;
/// DPP `SecurityLevel::CRITICAL` discriminant byte.
const SECURITY_LEVEL_CRITICAL: u8 = 1;
/// DPP `SecurityLevel::HIGH` discriminant byte.
const SECURITY_LEVEL_HIGH: u8 = 2;

/// Derive `key_count` identity-registration keypairs at DIP-9
/// paths and persist each one's private key to the Swift-owned
/// Keychain via the supplied callback handles.
///
/// On success, `out_pubkeys` is populated with the derived pubkeys
/// (and their paths) for the caller to build `IdentityPubkey` rows
/// for the subsequent registration call. The 32-byte ECDSA private
/// scalars are NOT included in the output — they were already
/// handed to the persister callback per iteration.
///
/// # Parameters
///
/// - `network`: selects the DIP-9 coin slot AND the WIF version
///   byte (the persister callback receives a `key_type` byte and
///   can derive WIF on the Swift side if needed).
/// - `wallet_id_bytes`: pointer to a 32-byte wallet id. Forwarded
///   to both callbacks. Required even though the resolver needs
///   it most — the persister callback receives it too so the
///   Swift impl can stamp `walletId` into Keychain metadata
///   without remembering which call this row came from.
/// - `identity_index`: BIP-9 identity index slot.
/// - `key_count`: number of consecutive `key_index` slots to
///   derive, starting at 0.
/// - `mnemonic_resolver_handle`: opaque handle from
///   [`crate::dash_sdk_mnemonic_resolver_create`].
/// - `persister_handle`: opaque handle from
///   [`crate::dash_sdk_identity_key_persister_create`].
/// - `out_pubkeys`: populated on success with a heap-allocated
///   array of pubkey-only rows. Release with
///   [`crate::dash_sdk_derive_identity_keys_from_mnemonic_free`]
///   — same memory layout as the lower-level FFI's output, so the
///   existing free path handles it correctly.
///
/// On error `*out_pubkeys` is left at the empty zero state. Any
/// keys persisted by partial success before the failing iteration
/// are NOT rolled back — the persister callback is single-shot
/// per key. The Swift caller is expected to surface the error and
/// not proceed to registration; orphan Keychain rows from a
/// partially-failed call are harmless (they're just unused
/// derived material).
///
/// # Safety
///
/// - `wallet_id_bytes` must be valid for 32 readable bytes.
/// - Both handle pointers must come from their respective `create`
///   functions and remain valid for the duration of the call.
/// - `out_pubkeys` must be a valid, writable
///   [`IdentityRegistrationKeyDerivationsFFI`] pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn dash_sdk_derive_and_persist_identity_keys(
    network: FFINetwork,
    wallet_id_bytes: *const u8,
    identity_index: u32,
    key_count: u32,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    persister_handle: *mut IdentityKeyPersisterHandle,
    out_pubkeys: *mut IdentityRegistrationKeyDerivationsFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_pubkeys);
    // Pre-zero so a failed call leaves the caller staring at a known
    // empty struct, never uninitialized memory.
    *out_pubkeys = IdentityRegistrationKeyDerivationsFFI {
        items: ptr::null_mut(),
        count: 0,
    };

    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(persister_handle);

    if key_count == 0 {
        return PlatformWalletFFIResult::ok();
    }

    // ---- Resolve mnemonic ----------------------------------------------------
    // Stack-resident, zeroized-on-drop buffer the resolver writes into.
    let mut mnemonic_buf: Zeroizing<[u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]> =
        Zeroizing::new([0u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]);
    let mut mnemonic_len: usize = 0;

    let resolver = &*mnemonic_resolver_handle;
    let resolver_vtable = &*resolver.vtable;
    let rc = (resolver_vtable.resolve)(
        resolver.ctx as *const c_void,
        wallet_id_bytes,
        mnemonic_buf.as_mut_ptr() as *mut c_char,
        MNEMONIC_RESOLVER_BUFFER_CAPACITY,
        &mut mnemonic_len,
    );
    match rc {
        x if x == mnemonic_resolver_result::SUCCESS => {}
        x if x == mnemonic_resolver_result::NOT_FOUND => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic resolver: no mnemonic stored for the supplied wallet_id",
            );
        }
        x if x == mnemonic_resolver_result::BUFFER_TOO_SMALL => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic resolver: mnemonic exceeded the FFI buffer capacity",
            );
        }
        _ => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic resolver: failed (other / Keychain access error)",
            );
        }
    }
    if mnemonic_len == 0 || mnemonic_len > MNEMONIC_RESOLVER_BUFFER_CAPACITY {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "mnemonic resolver: returned invalid length",
        );
    }

    // Validate UTF-8 once over the prefix the resolver claimed to
    // write. Done in-place so we never construct a `String`
    // (Swift's String can't be zeroized; ours can).
    let mnemonic_str = unwrap_result_or_return!(std::str::from_utf8(&mnemonic_buf[..mnemonic_len]));
    let mnemonic = unwrap_result_or_return!(parse_mnemonic_any_language(mnemonic_str));

    // ---- Derive seed + master xpriv ------------------------------------------
    // Empty passphrase to mirror the rest of the SDK (no caller
    // surface for a BIP-39 passphrase yet).
    let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(""));
    // Mnemonic is no longer needed; explicit drop releases its
    // (non-zeroized) `String` storage early.
    drop(mnemonic);

    let kw_network: Network = network.into();
    let master = unwrap_result_or_return!(ExtendedPrivKey::new_master(kw_network, seed.as_ref()));
    let secp = Secp256k1::new();

    // ---- Walk derivation paths, persist, build pubkey-only rows --------------
    let persister = &*persister_handle;
    let persister_vtable = &*persister.vtable;

    let mut rows: Vec<IdentityKeyPreviewFFI> = Vec::with_capacity(key_count as usize);

    // Hand-roll cleanup: each successfully-pushed row owns a
    // CString allocation (path) + a heap-allocated pubkey buffer
    // that won't be freed by `Vec::drop`.
    let cleanup = |rows: Vec<IdentityKeyPreviewFFI>| {
        for row in rows {
            if !row.derivation_path.is_null() {
                let _ = CString::from_raw(row.derivation_path);
            }
            if !row.public_key.is_null() && row.public_key_len > 0 {
                let _ = Vec::from_raw_parts(row.public_key, row.public_key_len, row.public_key_len);
            }
            // private_key_bytes is inline zeros; no WIF allocated.
        }
    };

    for key_index in 0..key_count {
        let path = match identity_auth_derivation_path(kw_network, identity_index, key_index) {
            Ok(p) => p,
            Err(detail) => {
                cleanup(rows);
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    format!(
                        "derive_and_persist: path build failed at \
                         (identity={identity_index}, key={key_index}): {detail}"
                    ),
                );
            }
        };

        let derived = match master.derive_priv(&secp, &path) {
            Ok(d) => d,
            Err(e) => {
                cleanup(rows);
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    format!(
                        "derive_and_persist: derive_priv failed at \
                         (identity={identity_index}, key={key_index}): {e}"
                    ),
                );
            }
        };

        // Materialize pubkey + hash160 once.
        let extended_pub = ExtendedPubKey::from_priv(&secp, &derived);
        let pub_bytes: [u8; 33] = extended_pub.public_key.serialize();
        let pub_hash: [u8; 20] = dashcore::hashes::hash160::Hash::hash(&pub_bytes).to_byte_array();

        // Hold the secret scalar in a buffer that drops with
        // `zeroize::Zeroize::zeroize` at scope end.
        let mut priv_scalar: [u8; 32] = derived.private_key.secret_bytes();

        let path_cstring = match CString::new(path.to_string()) {
            Ok(s) => s,
            Err(e) => {
                priv_scalar.zeroize();
                cleanup(rows);
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                    format!("derivation path contained NUL byte: {e}"),
                );
            }
        };

        // Per-key DPP metadata. Canonical default identity layout,
        // extending rs-dpp's `main_keys_with_random_authentication_keys_*`
        // (MASTER → CRITICAL → HIGH) with a dedicated TRANSFER key:
        //   0 → AUTHENTICATION / MASTER   (signs IdentityUpdate / IdentityCreate)
        //   1 → AUTHENTICATION / CRITICAL (signs token state transitions —
        //       `combined_security_level_requirement` collapses any batch
        //       containing a token transition to `[CRITICAL]`; without a
        //       CRITICAL key the identity cannot mint / burn / freeze tokens)
        //   2 → AUTHENTICATION / HIGH     (general document operations)
        //   3 → TRANSFER / CRITICAL       (signs IdentityCreditTransfer /
        //       IdentityCreditWithdrawal; without it those broadcasts fail
        //       on-chain with "no transfer public key")
        //   > 3 → AUTHENTICATION / HIGH   (any further keys)
        //
        // Slots 0/1/2 keep their pre-existing (purpose, security_level)
        // bytes byte-for-byte, so a caller still passing key_count == 3
        // is unchanged. TRANSFER keys take no contract bounds, and this
        // function never sets bounds for any key.
        let (purpose, security_level) = match key_index {
            0 => (PURPOSE_AUTHENTICATION, SECURITY_LEVEL_MASTER),
            1 => (PURPOSE_AUTHENTICATION, SECURITY_LEVEL_CRITICAL),
            2 => (PURPOSE_AUTHENTICATION, SECURITY_LEVEL_HIGH),
            3 => (PURPOSE_TRANSFER, SECURITY_LEVEL_CRITICAL),
            _ => (PURPOSE_AUTHENTICATION, SECURITY_LEVEL_HIGH),
        };
        let args = PersistKeyArgs {
            wallet_id_bytes,
            identity_index,
            key_id: key_index,
            key_index,
            derivation_path_cstr: path_cstring.as_ptr(),
            public_key_bytes: pub_bytes.as_ptr(),
            public_key_len: pub_bytes.len(),
            public_key_hash_bytes: pub_hash.as_ptr(),
            private_key_bytes: priv_scalar.as_ptr(),
            key_type: KEY_TYPE_ECDSA_SECP256K1,
            purpose,
            security_level,
        };
        let persist_rc =
            (persister_vtable.persist_key)(persister.ctx as *const c_void, &args as *const _);

        // Scrub the secret scalar before doing anything else
        // (success or failure). The persister callback is
        // contractually NOT allowed to retain the pointer past
        // its return, so zeroizing here is safe.
        priv_scalar.zeroize();

        if persist_rc != PERSIST_KEY_SUCCESS {
            // Path CString hasn't been transferred to the row yet;
            // it falls out of scope here.
            drop(path_cstring);
            cleanup(rows);
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!(
                    "persister callback returned false at \
                     (identity={identity_index}, key={key_index})"
                ),
            );
        }

        // Detach the pubkey buffer for the FFI return.
        let mut pub_box: Box<[u8]> = pub_bytes.to_vec().into_boxed_slice();
        let pub_ptr = pub_box.as_mut_ptr();
        let pub_len = pub_box.len();
        std::mem::forget(pub_box);

        rows.push(IdentityKeyPreviewFFI {
            identity_index,
            derivation_path: path_cstring.into_raw(),
            public_key: pub_ptr,
            public_key_len: pub_len,
            // Intentionally null — the private key was already
            // shipped to the persister callback. The free function
            // tolerates null.
            private_key_wif: ptr::null_mut(),
            private_key_bytes: [0u8; 32],
        });
    }

    // Hand the rows off as a heap allocation. Same memory layout
    // as `dash_sdk_derive_identity_keys_from_mnemonic` produces —
    // releasable with the same `_free` function.
    let mut boxed_items = rows.into_boxed_slice();
    let items_ptr = boxed_items.as_mut_ptr();
    let items_count = boxed_items.len();
    std::mem::forget(boxed_items);

    *out_pubkeys = IdentityRegistrationKeyDerivationsFFI {
        items: items_ptr,
        count: items_count,
    };

    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derive_and_persist_callbacks::{
        dash_sdk_identity_key_persister_create, dash_sdk_identity_key_persister_destroy,
    };
    use rs_sdk_ffi::{dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy};
    use std::ffi::CStr;
    use std::sync::Mutex;

    /// English BIP-39 test vector (all-zero entropy). Same fixture
    /// the upstream `bip39` crate uses; reused here so the
    /// derivation produces a known seed.
    const ENGLISH_PHRASE: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[derive(Debug, Clone)]
    struct CapturedPersist {
        identity_index: u32,
        key_id: u32,
        key_index: u32,
        path: String,
        public_key: Vec<u8>,
        public_key_hash: Vec<u8>,
        private_key: Vec<u8>,
        key_type: u8,
        purpose: u8,
        security_level: u8,
        wallet_id: [u8; 32],
    }

    /// Per-test capture buffer the persister callback writes
    /// into. Each test allocates its own `Box<CaptureCtx>` and
    /// passes the raw pointer as the persister `ctx`, so tests
    /// running in parallel cannot stomp each other (cargo runs
    /// tests in the same module concurrently by default).
    struct CaptureCtx {
        rows: Mutex<Vec<CapturedPersist>>,
    }

    unsafe extern "C" fn capturing_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        let phrase = ENGLISH_PHRASE.as_bytes();
        if phrase.len() + 1 > out_capacity {
            return mnemonic_resolver_result::BUFFER_TOO_SMALL;
        }
        std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_buf.add(phrase.len()) = 0;
        *out_len = phrase.len();
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}

    /// Persister that writes to the per-test [`CaptureCtx`]. The
    /// `ctx` pointer is the `Box::into_raw` of a `CaptureCtx`
    /// allocated by the test.
    unsafe extern "C" fn capturing_persist(ctx: *const c_void, args: *const PersistKeyArgs) -> u8 {
        if ctx.is_null() {
            return crate::derive_and_persist_callbacks::PERSIST_KEY_FAILURE;
        }
        let capture = &*(ctx as *const CaptureCtx);
        let a = &*args;
        let path = CStr::from_ptr(a.derivation_path_cstr)
            .to_string_lossy()
            .into_owned();
        let public_key = std::slice::from_raw_parts(a.public_key_bytes, a.public_key_len).to_vec();
        let public_key_hash = std::slice::from_raw_parts(a.public_key_hash_bytes, 20).to_vec();
        let private_key = std::slice::from_raw_parts(a.private_key_bytes, 32).to_vec();
        let mut wallet_id = [0u8; 32];
        std::ptr::copy_nonoverlapping(a.wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
        capture.rows.lock().unwrap().push(CapturedPersist {
            identity_index: a.identity_index,
            key_id: a.key_id,
            key_index: a.key_index,
            path,
            public_key,
            public_key_hash,
            private_key,
            key_type: a.key_type,
            purpose: a.purpose,
            security_level: a.security_level,
            wallet_id,
        });
        PERSIST_KEY_SUCCESS
    }

    unsafe extern "C" fn failing_persist(_ctx: *const c_void, _args: *const PersistKeyArgs) -> u8 {
        crate::derive_and_persist_callbacks::PERSIST_KEY_FAILURE
    }

    /// Allocate a fresh capture context + paired handles.
    /// Returns `(resolver, persister, capture_box_ptr)` — the
    /// capture pointer must be freed (via `Box::from_raw`) after
    /// the test's assertions have run.
    fn make_capturing_handles() -> (
        *mut MnemonicResolverHandle,
        *mut IdentityKeyPersisterHandle,
        *mut CaptureCtx,
    ) {
        let capture = Box::into_raw(Box::new(CaptureCtx {
            rows: Mutex::new(Vec::new()),
        }));
        unsafe {
            (
                dash_sdk_mnemonic_resolver_create(
                    std::ptr::null_mut(),
                    capturing_resolve,
                    noop_destroy,
                ),
                dash_sdk_identity_key_persister_create(
                    capture as *mut c_void,
                    capturing_persist,
                    noop_destroy,
                ),
                capture,
            )
        }
    }

    /// Allocate handles where the persister always returns
    /// failure. No capture buffer is needed.
    fn make_failing_handles() -> (*mut MnemonicResolverHandle, *mut IdentityKeyPersisterHandle) {
        unsafe {
            (
                dash_sdk_mnemonic_resolver_create(
                    std::ptr::null_mut(),
                    capturing_resolve,
                    noop_destroy,
                ),
                dash_sdk_identity_key_persister_create(
                    std::ptr::null_mut(),
                    failing_persist,
                    noop_destroy,
                ),
            )
        }
    }

    #[test]
    fn happy_path_persists_three_keys_and_returns_pubkeys() {
        let (resolver, persister, capture) = make_capturing_handles();
        let wallet_id = [42u8; 32];
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let rc = unsafe {
            dash_sdk_derive_and_persist_identity_keys(
                FFINetwork::Testnet,
                wallet_id.as_ptr(),
                7, // identity_index
                3, // key_count
                resolver,
                persister,
                &mut out,
            )
        };
        assert_eq!(rc.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.count, 3);

        // SAFETY: `capture` is alive (we're about to free it
        // below) and the persister can no longer fire — the FFI
        // call has already returned synchronously.
        let captured = unsafe { (*capture).rows.lock().unwrap().clone() };
        assert_eq!(captured.len(), 3);

        // key_id, key_index sequence
        for (i, row) in captured.iter().enumerate() {
            assert_eq!(row.identity_index, 7);
            assert_eq!(row.key_id, i as u32);
            assert_eq!(row.key_index, i as u32);
            assert_eq!(row.key_type, KEY_TYPE_ECDSA_SECP256K1);
            assert_eq!(row.purpose, PURPOSE_AUTHENTICATION);
            assert_eq!(
                row.security_level,
                match i {
                    0 => SECURITY_LEVEL_MASTER,
                    1 => SECURITY_LEVEL_CRITICAL,
                    _ => SECURITY_LEVEL_HIGH,
                }
            );
            assert_eq!(row.public_key.len(), 33);
            assert_eq!(row.public_key_hash.len(), 20);
            assert_eq!(row.private_key.len(), 32);
            assert_eq!(row.wallet_id, wallet_id);
            // hash160 sanity check: matches what the helper would compute.
            let expected_hash =
                dashcore::hashes::hash160::Hash::hash(&row.public_key).to_byte_array();
            assert_eq!(row.public_key_hash.as_slice(), expected_hash.as_slice());
            // Path shape: testnet coin = 1.
            assert_eq!(row.path, format!("m/9'/1'/5'/0'/0'/7'/{}'", i));
        }

        unsafe {
            crate::dash_sdk_derive_identity_keys_from_mnemonic_free(&mut out);
            dash_sdk_mnemonic_resolver_destroy(resolver);
            dash_sdk_identity_key_persister_destroy(persister);
            // Reclaim the per-test capture box.
            let _ = Box::from_raw(capture);
        }
    }

    #[test]
    fn key_count_four_includes_transfer_critical_key() {
        let (resolver, persister, capture) = make_capturing_handles();
        let wallet_id = [42u8; 32];
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let rc = unsafe {
            dash_sdk_derive_and_persist_identity_keys(
                FFINetwork::Testnet,
                wallet_id.as_ptr(),
                7, // identity_index
                4, // key_count — now provisions the TRANSFER key at id 3
                resolver,
                persister,
                &mut out,
            )
        };
        assert_eq!(rc.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.count, 4);

        let captured = unsafe { (*capture).rows.lock().unwrap().clone() };
        assert_eq!(captured.len(), 4);

        // Slots 0/1/2 are unchanged from the legacy 3-key layout:
        // AUTHENTICATION with MASTER / CRITICAL / HIGH respectively.
        for (i, expected_level) in [
            SECURITY_LEVEL_MASTER,
            SECURITY_LEVEL_CRITICAL,
            SECURITY_LEVEL_HIGH,
        ]
        .iter()
        .enumerate()
        {
            assert_eq!(captured[i].key_type, KEY_TYPE_ECDSA_SECP256K1);
            assert_eq!(captured[i].purpose, PURPOSE_AUTHENTICATION);
            assert_eq!(captured[i].security_level, *expected_level);
        }

        // Slot 3 is the new TRANSFER / CRITICAL key required to sign
        // IdentityCreditTransfer (ID-04) / IdentityCreditWithdrawal
        // (ID-10) — without it those broadcasts fail on-chain with
        // "no transfer public key".
        let transfer = &captured[3];
        assert_eq!(transfer.key_id, 3);
        assert_eq!(transfer.key_index, 3);
        assert_eq!(transfer.key_type, KEY_TYPE_ECDSA_SECP256K1);
        assert_eq!(transfer.purpose, PURPOSE_TRANSFER);
        assert_eq!(transfer.security_level, SECURITY_LEVEL_CRITICAL);
        assert_eq!(transfer.public_key.len(), 33);
        assert_eq!(transfer.private_key.len(), 32);
        assert_eq!(transfer.path, "m/9'/1'/5'/0'/0'/7'/3'");

        unsafe {
            crate::dash_sdk_derive_identity_keys_from_mnemonic_free(&mut out);
            dash_sdk_mnemonic_resolver_destroy(resolver);
            dash_sdk_identity_key_persister_destroy(persister);
            let _ = Box::from_raw(capture);
        }
    }

    #[test]
    fn returning_false_from_persister_aborts_with_wallet_op_error() {
        let (resolver, persister) = make_failing_handles();
        let wallet_id = [1u8; 32];
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let rc = unsafe {
            dash_sdk_derive_and_persist_identity_keys(
                FFINetwork::Testnet,
                wallet_id.as_ptr(),
                0,
                2,
                resolver,
                persister,
                &mut out,
            )
        };
        assert_eq!(rc.code, PlatformWalletFFIResultCode::ErrorWalletOperation);
        assert_eq!(out.count, 0);
        assert!(out.items.is_null());
        unsafe {
            dash_sdk_mnemonic_resolver_destroy(resolver);
            dash_sdk_identity_key_persister_destroy(persister);
        }
    }

    #[test]
    fn rejects_null_inputs() {
        let (resolver, persister, capture) = make_capturing_handles();
        let wallet_id = [0u8; 32];
        // Null out_pubkeys.
        let rc = unsafe {
            dash_sdk_derive_and_persist_identity_keys(
                FFINetwork::Testnet,
                wallet_id.as_ptr(),
                0,
                1,
                resolver,
                persister,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(rc.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        // Null wallet id.
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let rc = unsafe {
            dash_sdk_derive_and_persist_identity_keys(
                FFINetwork::Testnet,
                std::ptr::null(),
                0,
                1,
                resolver,
                persister,
                &mut out,
            )
        };
        assert_eq!(rc.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        unsafe {
            dash_sdk_mnemonic_resolver_destroy(resolver);
            dash_sdk_identity_key_persister_destroy(persister);
            let _ = Box::from_raw(capture);
        }
    }

    #[test]
    fn key_count_zero_is_success_no_calls() {
        let (resolver, persister, capture) = make_capturing_handles();
        let wallet_id = [0u8; 32];
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let rc = unsafe {
            dash_sdk_derive_and_persist_identity_keys(
                FFINetwork::Testnet,
                wallet_id.as_ptr(),
                0,
                0,
                resolver,
                persister,
                &mut out,
            )
        };
        assert_eq!(rc.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.count, 0);
        assert_eq!(unsafe { (*capture).rows.lock().unwrap().len() }, 0);
        unsafe {
            dash_sdk_mnemonic_resolver_destroy(resolver);
            dash_sdk_identity_key_persister_destroy(persister);
            let _ = Box::from_raw(capture);
        }
    }
}
