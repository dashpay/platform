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

use std::ffi::CString;

use dashcore::PrivateKey as DashPrivateKey;
use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;
use rs_sdk_ffi::DashSDKNetwork;
use zeroize::Zeroizing;

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
            drop(Vec::from_raw_parts(pub_ptr, pub_len, pub_len));
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
            drop(Vec::from_raw_parts(pub_ptr, pub_len, pub_len));
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
    let mut private_key_bytes = [0u8; 32];
    private_key_bytes.copy_from_slice(derived.private_key.as_ref());

    *out_row = IdentityKeyPreviewFFI {
        identity_index,
        derivation_path: path_cstring.into_raw(),
        public_key: pub_ptr,
        public_key_len: pub_len,
        private_key_wif: wif_cstring.into_raw(),
        private_key_bytes,
    };

    PlatformWalletFFIResult::Success
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
        let _ = Vec::from_raw_parts(row.public_key, row.public_key_len, row.public_key_len);
        row.public_key = std::ptr::null_mut();
        row.public_key_len = 0;
    }
    if !row.private_key_wif.is_null() {
        // Overwrite the WIF chars in place before releasing the
        // CString allocation. Matches the loop variant's scrub.
        let len = std::ffi::CStr::from_ptr(row.private_key_wif)
            .to_bytes()
            .len();
        std::ptr::write_bytes(row.private_key_wif as *mut u8, 0, len);
        let _ = CString::from_raw(row.private_key_wif);
        row.private_key_wif = std::ptr::null_mut();
    }
    // Inline 32-byte secret — zero in place. The struct itself is
    // owned by the caller, so we don't free anything here.
    for byte in row.private_key_bytes.iter_mut() {
        *byte = 0;
    }
    row.identity_index = 0;
}
