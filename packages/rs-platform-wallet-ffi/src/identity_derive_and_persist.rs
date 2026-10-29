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
//! The key layout (key id, DIP-9 key index, key type, purpose, security
//! level) comes from [`platform_wallet::IdentityCreationKeyLayout`]; the
//! caller only picks which layout its flow registers. The persister
//! receives each key's DPP discriminant bytes and the Swift caller writes
//! whatever it is told. Contract bounds (the DashPay ENCRYPTION /
//! DECRYPTION keys) are not part of the persister row; read them from
//! [`crate::platform_wallet_identity_creation_key_layout`].
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
use key_wallet::bip32::{ExtendedPrivKey, ExtendedPubKey};
use platform_wallet::IdentityCreationKeyLayout;
use zeroize::{Zeroize, Zeroizing};

use crate::derive_and_persist_callbacks::{
    IdentityKeyPersisterHandle, PersistKeyArgs, PERSIST_KEY_SUCCESS,
};
use crate::error::*;
use crate::identity_creation_key_layout::IdentityCreationKeyLayoutFFI;
use crate::identity_key_preview::IdentityKeyPreviewFFI;
use crate::identity_keys_from_mnemonic::{
    identity_auth_derivation_path, parse_mnemonic_any_language,
};
use crate::identity_registration_with_signer::IdentityRegistrationKeyDerivationsFFI;
use crate::{check_ptr, unwrap_result_or_return};
use rs_sdk_ffi::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};

/// Derive the identity-registration keypairs of `layout` at their DIP-9
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
/// - `layout`: which [`platform_wallet::IdentityCreationKeyLayout`] the
///   flow registers. One key is derived and persisted per slot, in key-id
///   order.
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
    layout: IdentityCreationKeyLayoutFFI,
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

    let slots = IdentityCreationKeyLayout::from(layout).slots();

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

    // ---- Walk derivation paths, persist, build pubkey-only rows --------------
    let persister = &*persister_handle;
    let persister_vtable = &*persister.vtable;

    let mut rows: Vec<IdentityKeyPreviewFFI> = Vec::with_capacity(slots.len());

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

    for slot in &slots {
        let key_index = slot.derivation_key_index();
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

        let derived = match master.derive_priv(&path) {
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
        let extended_pub = ExtendedPubKey::from_priv(&derived);
        let pub_bytes: [u8; 33] = extended_pub.public_key.to_compressed();
        let pub_hash: [u8; 20] = dashcore::hashes::hash160::Hash::hash(&pub_bytes).to_byte_array();

        // Hold the secret scalar in a buffer that drops with
        // `zeroize::Zeroize::zeroize` at scope end.
        let mut priv_scalar: [u8; 32] = *derived.private_key.to_bytes();

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

        let args = PersistKeyArgs {
            wallet_id_bytes,
            identity_index,
            key_id: slot.key_id,
            key_index,
            derivation_path_cstr: path_cstring.as_ptr(),
            public_key_bytes: pub_bytes.as_ptr(),
            public_key_len: pub_bytes.len(),
            public_key_hash_bytes: pub_hash.as_ptr(),
            private_key_bytes: priv_scalar.as_ptr(),
            key_type: slot.key_type as u8,
            purpose: slot.purpose as u8,
            security_level: slot.security_level as u8,
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
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
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

    /// Derive + persist `layout` at identity index 7 on testnet and return
    /// the captured persister rows plus the returned pubkeys.
    fn derive_and_capture(
        layout: IdentityCreationKeyLayoutFFI,
    ) -> (Vec<CapturedPersist>, Vec<Vec<u8>>) {
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
                layout,
                resolver,
                persister,
                &mut out,
            )
        };
        assert_eq!(rc.code, PlatformWalletFFIResultCode::Success);

        // SAFETY: `capture` is alive (freed below) and the persister can no
        // longer fire — the FFI call has already returned synchronously.
        let captured = unsafe { (*capture).rows.lock().unwrap().clone() };
        let pubkeys = unsafe { std::slice::from_raw_parts(out.items, out.count) }
            .iter()
            .map(|row| unsafe {
                std::slice::from_raw_parts(row.public_key, row.public_key_len).to_vec()
            })
            .collect();
        for row in &captured {
            assert_eq!(row.identity_index, 7);
            assert_eq!(row.wallet_id, wallet_id);
            assert_eq!(row.public_key.len(), 33);
            assert_eq!(row.private_key.len(), 32);
            let expected_hash =
                dashcore::hashes::hash160::Hash::hash(&row.public_key).to_byte_array();
            assert_eq!(row.public_key_hash.as_slice(), expected_hash.as_slice());
            // Derivation slot == key id; testnet coin = 1.
            assert_eq!(row.key_index, row.key_id);
            assert_eq!(row.path, format!("m/9'/1'/5'/0'/0'/7'/{}'", row.key_id));
        }

        unsafe {
            crate::dash_sdk_derive_identity_keys_from_mnemonic_free(&mut out);
            dash_sdk_mnemonic_resolver_destroy(resolver);
            dash_sdk_identity_key_persister_destroy(persister);
            let _ = Box::from_raw(capture);
        }
        (captured, pubkeys)
    }

    /// The `(key_id, key_type, purpose, security_level)` bytes the Rust
    /// layout assigns.
    fn layout_roles(layout: IdentityCreationKeyLayoutFFI) -> Vec<(u32, u8, u8, u8)> {
        IdentityCreationKeyLayout::from(layout)
            .slots()
            .iter()
            .map(|s| {
                (
                    s.key_id,
                    s.key_type as u8,
                    s.purpose as u8,
                    s.security_level as u8,
                )
            })
            .collect()
    }

    fn captured_roles(captured: &[CapturedPersist]) -> Vec<(u32, u8, u8, u8)> {
        captured
            .iter()
            .map(|r| (r.key_id, r.key_type, r.purpose, r.security_level))
            .collect()
    }

    #[test]
    fn should_persist_every_key_of_the_rust_layout_with_its_role() {
        for layout in [
            IdentityCreationKeyLayoutFFI::NewIdentity,
            IdentityCreationKeyLayoutFFI::NewIdentityWithDashPayKeys,
        ] {
            let (captured, pubkeys) = derive_and_capture(layout);
            assert_eq!(
                captured_roles(&captured),
                layout_roles(layout),
                "{layout:?}"
            );
            let persisted: Vec<Vec<u8>> = captured.iter().map(|r| r.public_key.clone()).collect();
            assert_eq!(pubkeys, persisted, "{layout:?}");
        }
    }

    #[test]
    fn should_derive_the_same_base_keys_with_and_without_dashpay_keys() {
        // Ids 0-3 derive the same keys with the same roles in both layouts,
        // at DIP-9 key index = key id; the DashPay layout adds ids 4-5.
        let (base, _) = derive_and_capture(IdentityCreationKeyLayoutFFI::NewIdentity);
        let (with_dashpay, _) =
            derive_and_capture(IdentityCreationKeyLayoutFFI::NewIdentityWithDashPayKeys);
        assert_eq!((base.len(), with_dashpay.len()), (4, 6));
        for (old, new) in base.iter().zip(&with_dashpay) {
            assert_eq!(old.key_id, new.key_id);
            assert_eq!(old.public_key, new.public_key);
            assert_eq!(
                (old.purpose, old.security_level),
                (new.purpose, new.security_level)
            );
        }
        assert_eq!(base[3].path, "m/9'/1'/5'/0'/0'/7'/3'");
        assert_eq!(
            [captured_roles(&base)[3]],
            [(
                3,
                KeyType::ECDSA_SECP256K1 as u8,
                Purpose::TRANSFER as u8,
                SecurityLevel::CRITICAL as u8
            )]
        );
        assert_eq!(
            (with_dashpay[4].purpose, with_dashpay[5].purpose),
            (Purpose::ENCRYPTION as u8, Purpose::DECRYPTION as u8)
        );
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
                IdentityCreationKeyLayoutFFI::NewIdentity,
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
                IdentityCreationKeyLayoutFFI::NewIdentity,
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
                IdentityCreationKeyLayoutFFI::NewIdentity,
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
}
