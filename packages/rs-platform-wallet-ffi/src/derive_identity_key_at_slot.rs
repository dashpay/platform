//! Single-slot mnemonic-driven identity-authentication key derivation.
//!
//! Companion to
//! [`crate::identity_keys_from_mnemonic::dash_sdk_derive_identity_keys_from_mnemonic`]
//! — same derivation logic, but returns ONE row at an arbitrary
//! `(identity_index, key_index)` slot instead of `0..key_count`.
//!
//! Use case: adding a new key to an existing identity. The Swift
//! caller picks `key_index = max(existing_key_ids) + 1`, calls this
//! function to derive the keypair, persists the private bytes to
//! Keychain, then submits an `updateIdentity(addPublicKeys:)` state
//! transition with the returned public key. The mnemonic-driven
//! shape (rather than wallet-handle) keeps watch-only wallets
//! supported — Rust never needs an in-process xpriv loaded.
//!
//! Derivation passes through the library function
//! [`platform_wallet::wallet::identity::network::identity_handle::
//! derive_ecdsa_identity_auth_keypair_from_master`] so the path
//! builder + secp256k1 derive are not duplicated here. Only the
//! C-ABI marshalling lives in this file.

use std::ffi::{c_void, CString};
use std::os::raw::c_char;

use dashcore::PrivateKey as DashPrivateKey;
use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;
use rs_sdk_ffi::DashSDKNetwork;
use zeroize::Zeroizing;

use crate::derive_and_persist_callbacks::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};
use crate::error::*;
use crate::identity_key_preview::IdentityKeyPreviewFFI;
use crate::identity_keys_from_mnemonic::{map_network, parse_mnemonic_any_language};

/// Derive a single ECDSA identity-authentication keypair at
/// `(identity_index, key_index)` from a BIP-39 mnemonic. Returns
/// the row in `*out_row`; release with
/// [`dash_sdk_derive_identity_key_at_slot_free`].
///
/// Currently ECDSA-only — `key_type` callers (BLS, EdDSA) need a
/// different derivation curve and aren't wired through yet. The
/// caller chooses ECDSA-secp256k1 vs ECDSA-hash160 at the
/// `IdentityPublicKey` construction step (downstream of this
/// function) since both share the same DIP-9 path and the same
/// 33-byte compressed public key bytes.
///
/// # Parameters
/// - `mnemonic_cstr` / `passphrase_cstr`: the BIP-39 inputs.
///   `passphrase_cstr` may be NULL.
/// - `network`: selects the DIP-9 coin-type slot (mainnet vs testnet).
/// - `identity_index`: hardened identity index slot.
/// - `key_index`: hardened key index slot. The caller chooses this
///   (typically `max(existing_key_ids) + 1`).
/// - `out_row`: populated on success with one
///   [`IdentityKeyPreviewFFI`]. Release with the paired free
///   function.
/// - `out_error`: populated on failure with the usual error detail.
///
/// On error `*out_row` is left zeroed.
///
/// # Safety
/// - `mnemonic_cstr` must be a valid, NUL-terminated UTF-8 C string
///   for the duration of the call.
/// - `passphrase_cstr` may be NULL; otherwise must be valid + NUL-
///   terminated.
/// - `out_row` must be a valid, writable pointer.
/// - `out_error` may be NULL.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_identity_key_at_slot(
    mnemonic_cstr: *const std::os::raw::c_char,
    passphrase_cstr: *const std::os::raw::c_char,
    network: DashSDKNetwork,
    identity_index: u32,
    key_index: u32,
    out_row: *mut IdentityKeyPreviewFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    use std::ffi::CStr;

    if out_row.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "out_row is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    // Pre-zero so a failed call leaves the caller staring at known
    // empty state, never uninitialized memory. Mirrors the
    // mnemonic-loop variant's behaviour.
    *out_row = IdentityKeyPreviewFFI::empty();

    if mnemonic_cstr.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "mnemonic_cstr is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // ---- UTF-8 inputs --------------------------------------------------------
    let mnemonic_str = match CStr::from_ptr(mnemonic_cstr).to_str() {
        Ok(s) => s,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    format!("mnemonic_cstr is not valid UTF-8: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };
    let passphrase_str: &str = if passphrase_cstr.is_null() {
        ""
    } else {
        match CStr::from_ptr(passphrase_cstr).to_str() {
            Ok(s) => s,
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        format!("passphrase_cstr is not valid UTF-8: {e}"),
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        }
    };

    derive_at_slot_inner(
        mnemonic_str,
        passphrase_str,
        network,
        identity_index,
        key_index,
        out_row,
        out_error,
    )
}

/// Inner implementation shared by the mnemonic-string and resolver-
/// based entry points. Assumes the caller has already pre-zeroed
/// `*out_row` (so a failed call returns a known empty struct).
unsafe fn derive_at_slot_inner(
    mnemonic_str: &str,
    passphrase_str: &str,
    network: DashSDKNetwork,
    identity_index: u32,
    key_index: u32,
    out_row: *mut IdentityKeyPreviewFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    // ---- Mnemonic + seed -----------------------------------------------------
    let mnemonic = match parse_mnemonic_any_language(mnemonic_str) {
        Ok(m) => m,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("invalid mnemonic: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };
    // 64-byte seed wrapped in `Zeroizing` so it gets scrubbed when
    // this function returns (success or failure).
    let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(passphrase_str));

    // ---- Master xpriv --------------------------------------------------------
    let kw_network = map_network(network);
    let master = match ExtendedPrivKey::new_master(kw_network, seed.as_ref()) {
        Ok(m) => m,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorWalletOperation,
                    format!("ExtendedPrivKey::new_master failed: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorWalletOperation;
        }
    };

    // ---- Library-side derivation --------------------------------------------
    // The library does the actual path build + secp256k1 derive
    // pass; we only hand it the master and the slot indices.
    let derived = match derive_ecdsa_identity_auth_keypair_from_master(
        &master,
        kw_network,
        identity_index,
        key_index,
    ) {
        Ok(d) => d,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorWalletOperation,
                    format!("derivation failed: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorWalletOperation;
        }
    };

    // ---- Marshal into IdentityKeyPreviewFFI ---------------------------------
    let path_cstring = match CString::new(derived.derivation_path.to_string()) {
        Ok(s) => s,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    format!("derivation path contained NUL byte: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };

    let pub_bytes_vec = derived.public_key.to_vec();
    let mut pub_box: Box<[u8]> = pub_bytes_vec.into_boxed_slice();
    let pub_ptr = pub_box.as_mut_ptr();
    let pub_len = pub_box.len();
    std::mem::forget(pub_box);

    // WIF for the keychain-explorer / debugging UI. Build a temporary
    // secp256k1 SecretKey from the derived bytes — copying through
    // `DashPrivateKey` matches the loop variant's WIF shape.
    let secret_key = match dashcore::secp256k1::SecretKey::from_slice(derived.private_key.as_ref())
    {
        Ok(k) => k,
        Err(e) => {
            // Roll back the path / pubkey allocations before bailing.
            // Reclaim the boxed slice through the same allocator
            // shape it was created with (`into_boxed_slice` →
            // `Box<[u8]>`). Reconstructing a `Vec` here is UB
            // whenever `len < capacity` for the original source
            // vector.
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                pub_ptr, pub_len,
            )));
            drop(path_cstring);
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorWalletOperation,
                    format!("SecretKey::from_slice failed: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorWalletOperation;
        }
    };
    let dash_private = DashPrivateKey {
        compressed: true,
        network: kw_network,
        inner: secret_key,
    };
    let wif_cstring = match CString::new(dash_private.to_wif()) {
        Ok(s) => s,
        Err(e) => {
            // Reclaim the boxed slice through the same allocator
            // shape it was created with (`into_boxed_slice` →
            // `Box<[u8]>`). Reconstructing a `Vec` here is UB
            // whenever `len < capacity` for the original source
            // vector.
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                pub_ptr, pub_len,
            )));
            drop(path_cstring);
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    format!("WIF string contained NUL byte: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };

    // Copy the secret bytes out of `Zeroizing` into the FFI row.
    // The original `derived` value drops at scope exit and zeroizes
    // the source buffer; the FFI row's `private_key_bytes` is the
    // caller's responsibility to scrub via the paired `_free`.
    //
    // Stage the inline 32-byte copy through `Zeroizing<[u8; 32]>`
    // so the **stack** copy gets scrubbed when this function
    // returns. `[u8; 32]` is `Copy`, so writing into the FFI row
    // duplicates the bytes — the row's bytes are the caller's
    // problem, but the local buffer would otherwise linger on the
    // stack with the live secret until the frame is overwritten
    // by the next call.
    let mut private_key_bytes_buf: Zeroizing<[u8; 32]> = Zeroizing::new([0u8; 32]);
    private_key_bytes_buf.copy_from_slice(derived.private_key.as_ref());

    *out_row = IdentityKeyPreviewFFI {
        identity_index,
        derivation_path: path_cstring.into_raw(),
        public_key: pub_ptr,
        public_key_len: pub_len,
        private_key_wif: wif_cstring.into_raw(),
        private_key_bytes: *private_key_bytes_buf,
    };

    PlatformWalletFFIResult::Success
}

/// Resolver-based variant of
/// [`dash_sdk_derive_identity_key_at_slot`].
///
/// Replaces the raw-mnemonic entry point with the same callback
/// pattern that
/// [`crate::dash_sdk_derive_and_persist_identity_keys`] uses: the
/// caller hands in a [`MnemonicResolverHandle`] keyed by
/// `wallet_id_bytes`, and Rust pulls the BIP-39 mnemonic across
/// the FFI from Swift's iOS Keychain on demand. The mnemonic
/// never lives in a Swift `String` outside the resolver
/// trampoline's stack frame — closes the
/// `swift-sdk/CLAUDE.md` "no mnemonic round-tripping" rule that
/// the raw-cstring entry point still violates.
///
/// Use this in preference to
/// [`dash_sdk_derive_identity_key_at_slot`] from Swift. The
/// raw-cstring variant is retained for tests + any non-iOS caller
/// that already has the mnemonic in hand.
///
/// # Safety
/// - `wallet_id_bytes` must be valid for 32 readable bytes.
/// - `mnemonic_resolver_handle` must be a non-null handle
///   produced by `dash_sdk_mnemonic_resolver_create`, and remain
///   valid for the duration of the call.
/// - `out_row` must be a valid, writable pointer.
/// - `out_error` may be NULL.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_identity_key_at_slot_with_resolver(
    network: DashSDKNetwork,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    identity_index: u32,
    key_index: u32,
    out_row: *mut IdentityKeyPreviewFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_row.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "out_row is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    *out_row = IdentityKeyPreviewFFI::empty();

    if wallet_id_bytes.is_null() || mnemonic_resolver_handle.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "wallet_id_bytes and mnemonic_resolver_handle are required",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // Stack-resident, zeroized-on-drop buffer the resolver writes
    // into. Same shape as `dash_sdk_derive_and_persist_identity_keys`.
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
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorWalletOperation,
                    "mnemonic resolver: no mnemonic stored for the supplied wallet_id",
                );
            }
            return PlatformWalletFFIResult::ErrorWalletOperation;
        }
        x if x == mnemonic_resolver_result::BUFFER_TOO_SMALL => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorWalletOperation,
                    "mnemonic resolver: mnemonic exceeded the FFI buffer capacity",
                );
            }
            return PlatformWalletFFIResult::ErrorWalletOperation;
        }
        _ => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorWalletOperation,
                    "mnemonic resolver: failed (other / Keychain access error)",
                );
            }
            return PlatformWalletFFIResult::ErrorWalletOperation;
        }
    }

    let mnemonic_str = match std::str::from_utf8(&mnemonic_buf[..mnemonic_len]) {
        Ok(s) => s,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    format!("mnemonic resolver: not valid UTF-8: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };

    // Passphrase isn't part of the resolver vtable today; the
    // existing `dash_sdk_derive_and_persist_identity_keys` makes
    // the same assumption (BIP-39 wallets don't carry a passphrase
    // in this app). When that changes, extend the resolver to
    // surface it the same way the mnemonic is surfaced.
    derive_at_slot_inner(
        mnemonic_str,
        "",
        network,
        identity_index,
        key_index,
        out_row,
        out_error,
    )
}

/// Free a row populated by
/// [`dash_sdk_derive_identity_key_at_slot`]. Zeroes the inline
/// 32-byte secret + the WIF buffer in place before releasing the
/// allocations, matching the `_free` paired with the loop variant.
///
/// # Safety
/// `out_row` must point to a row populated by
/// [`dash_sdk_derive_identity_key_at_slot`] and must not have been
/// freed already.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_identity_key_at_slot_free(
    out_row: *mut IdentityKeyPreviewFFI,
) {
    if out_row.is_null() {
        return;
    }
    let row = &mut *out_row;

    if !row.derivation_path.is_null() {
        let _ = CString::from_raw(row.derivation_path);
        row.derivation_path = std::ptr::null_mut();
    }
    if !row.public_key.is_null() && row.public_key_len > 0 {
        // Mirror the allocator shape from
        // `dash_sdk_derive_identity_key_at_slot` — the buffer was
        // created via `Vec::into_boxed_slice` + `Box::into_raw`,
        // so `Box::from_raw(slice_from_raw_parts_mut(...))` is the
        // matching dispose. `Vec::from_raw_parts(ptr, len, len)`
        // is UB whenever the source vec had `cap > len`.
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            row.public_key,
            row.public_key_len,
        ));
        row.public_key = std::ptr::null_mut();
        row.public_key_len = 0;
    }
    if !row.private_key_wif.is_null() {
        // Reconstruct the owned `CString` first, THEN scrub through
        // its owned buffer. Earlier revisions zeroed via
        // `write_bytes(ptr, 0, strlen)` *before* `CString::from_raw`
        // — that is undefined behaviour: `CString::from_raw`
        // recomputes the length internally with `strlen`, so a
        // pre-zeroed buffer makes it free a 1-byte allocation
        // against the original `(len + 1)`-byte allocation
        // (rust-lang/rust#68456). Reconstructing first preserves
        // the original length in `CString::as_bytes().len()` so the
        // allocator sees a matching size on drop.
        let cstring = CString::from_raw(row.private_key_wif);
        let bytes_len = cstring.as_bytes().len();
        std::ptr::write_bytes(cstring.as_ptr() as *mut u8, 0, bytes_len);
        drop(cstring);
        row.private_key_wif = std::ptr::null_mut();
    }
    // Inline 32-byte secret — zero in place. The struct itself is
    // owned by the caller, so we don't free anything here.
    for byte in row.private_key_bytes.iter_mut() {
        *byte = 0;
    }
    row.identity_index = 0;
}
