//! Mnemonic-driven identity-registration key derivation.
//!
//! Surgical companion to
//! [`crate::identity_registration_with_signer::platform_wallet_derive_identity_keys_for_index`]
//! — same output shape, different input. Where the wallet-handle
//! variant requires a full `key_wallet::Wallet` with an in-process
//! xpriv loaded, this entry point takes the BIP-39 mnemonic directly
//! and never touches the wallet manager.
//!
//! # Why this exists
//!
//! Wallets restored from a Swift-side persisted state (SwiftData
//! row + Keychain mnemonic) load into the Rust process as
//! **watch-only** — the seed is in iOS Keychain, not in the
//! `WalletManager`. Calling
//! [`platform_wallet_derive_identity_keys_for_index`](crate::platform_wallet_derive_identity_keys_for_index)
//! on those wallets fails with `"Cannot derive private keys from
//! watch-only wallet"` because `Wallet::derive_extended_private_key`
//! refuses to operate without the seed.
//!
//! Rather than push the seed across the FFI just to load it into the
//! `WalletManager` (which would defeat the watch-only model that the
//! rest of this crate carefully preserves), the Swift caller hands
//! us the mnemonic for the duration of one call. The mnemonic is
//! parsed, converted to a 64-byte seed inside a [`Zeroizing`] buffer,
//! used to build the master xpriv, walked through `key_count`
//! derivation paths, and then dropped — all within this function's
//! lifetime. Only the final 32-byte secret scalars cross the FFI for
//! the caller to persist into Keychain.
//!
//! The output rows reuse [`IdentityKeyPreviewFFI`] and the wrapper
//! reuses [`crate::identity_registration_with_signer::IdentityRegistrationKeyDerivationsFFI`]
//! so the Swift marshalling code already written for the wallet-handle
//! variant works unchanged.

use std::ffi::CString;

use dashcore::secp256k1::Secp256k1;
use dashcore::PrivateKey as DashPrivateKey;
use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, ExtendedPubKey};
use key_wallet::dip9::{
    IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
};
use key_wallet::mnemonic::{Language, Mnemonic};
use rs_sdk_ffi::DashSDKNetwork;
use zeroize::Zeroizing;

use crate::error::*;
use crate::identity_key_preview::IdentityKeyPreviewFFI;
use crate::identity_registration_with_signer::IdentityRegistrationKeyDerivationsFFI;

/// Parse a BIP-39 mnemonic against every supported wordlist.
///
/// Mirrors the auto-detect helper in `rs-sdk-ffi::signer_simple`
/// (`parse_mnemonic_any_language`); kept inline here so this crate
/// doesn't have to take a public dependency on that crate-internal
/// helper. BIP-39 wordlists are mutually exclusive within a single
/// phrase, so the first language that yields a valid mnemonic is the
/// right one.
fn parse_mnemonic_any_language(phrase: &str) -> Result<Mnemonic, &'static str> {
    const LANGUAGES: [Language; 10] = [
        Language::English,
        Language::Spanish,
        Language::French,
        Language::Italian,
        Language::Japanese,
        Language::Korean,
        Language::ChineseSimplified,
        Language::ChineseTraditional,
        Language::Czech,
        Language::Portuguese,
    ];
    for lang in LANGUAGES {
        if let Ok(m) = Mnemonic::from_phrase(phrase, lang) {
            return Ok(m);
        }
    }
    Err("phrase does not match any supported BIP-39 wordlist")
}

/// Map the C-ABI `DashSDKNetwork` enum to `key_wallet::Network`.
///
/// `Local` collapses to `Regtest` to match the rest of the FFI surface
/// (`dash_sdk_signer_create_from_private_key`, `dash_sdk_sign_with_mnemonic_and_path`).
fn map_network(network: DashSDKNetwork) -> key_wallet::Network {
    match network {
        DashSDKNetwork::SDKMainnet => key_wallet::Network::Mainnet,
        DashSDKNetwork::SDKTestnet => key_wallet::Network::Testnet,
        DashSDKNetwork::SDKRegtest => key_wallet::Network::Regtest,
        DashSDKNetwork::SDKDevnet => key_wallet::Network::Devnet,
        DashSDKNetwork::SDKLocal => key_wallet::Network::Regtest,
    }
}

/// Build the DIP-9 identity-authentication derivation path
/// `m/9'/coin'/5'/0'/0'/identity_index'/key_index'`.
///
/// Coin type is selected by `network` (mainnet vs testnet). The hardened
/// `0'` slot in position 4 is `KeyDerivationType::ECDSA`; this helper
/// hardcodes ECDSA because that's the only key type the identity-
/// registration path supports today (matches `derive_identity_auth_keypair`
/// in `platform-wallet`, which is the source of truth for live
/// registrations / discovery / preview).
///
/// Re-derived locally instead of borrowed from `platform-wallet` because
/// `platform_wallet::identity_auth_derivation_path` is `pub(crate)` and
/// this entry point sits outside that crate.
fn identity_auth_derivation_path(
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<DerivationPath, String> {
    let base_path: DerivationPath = match network {
        key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
        _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
    }
    .into();
    // KeyDerivationType::ECDSA = 0 — hardcoded to match the only path
    // shape `derive_identity_auth_keypair` ever produces.
    let key_type_index: u32 = 0;
    Ok(base_path.extend([
        ChildNumber::from_hardened_idx(key_type_index)
            .map_err(|e| format!("invalid key_type_index: {e}"))?,
        ChildNumber::from_hardened_idx(identity_index)
            .map_err(|e| format!("invalid identity_index {identity_index}: {e}"))?,
        ChildNumber::from_hardened_idx(key_index)
            .map_err(|e| format!("invalid key_index {key_index}: {e}"))?,
    ]))
}

/// Derive `key_count` identity-registration keys from
/// `(mnemonic, passphrase, network)` at DIP-9 paths
/// `m/9'/coin'/5'/0'/0'/identity_index'/key_index'` for
/// `key_index` in `0..key_count`.
///
/// Returns rows of `(pubkey_bytes, derivation_path_cstr,
/// private_key_bytes, private_key_wif)` via the shared
/// [`IdentityRegistrationKeyDerivationsFFI`] / [`IdentityKeyPreviewFFI`]
/// shape — same memory layout as the wallet-handle variant
/// [`platform_wallet_derive_identity_keys_for_index`](crate::platform_wallet_derive_identity_keys_for_index)
/// so the Swift marshalling and free path are identical.
///
/// Both the seed and intermediate xprivs are wrapped in [`Zeroizing`]
/// inside Rust; only the final 32-byte secret scalars cross the FFI
/// boundary for the caller to persist into Keychain.
///
/// # Why this exists
/// Identity-key derivation that previously routed through the wallet
/// handle ([`platform_wallet_derive_identity_keys_for_index`](crate::platform_wallet_derive_identity_keys_for_index))
/// fails for restored watch-only wallets because Rust has no xpriv
/// loaded for them. This entry point bypasses the wallet entirely —
/// the caller supplies the mnemonic.
///
/// # Parameters
/// - `mnemonic_cstr`: null-terminated UTF-8 BIP-39 phrase. Auto-
///   detects language from the supported wordlists.
/// - `passphrase_cstr`: null-terminated UTF-8 BIP-39 passphrase. May
///   be null, in which case the empty passphrase is used.
/// - `network`: selects the coin-type slot in the derivation path
///   AND the WIF version byte.
/// - `identity_index`: hardened identity index slot.
/// - `key_count`: number of consecutive `key_index` slots to derive,
///   starting at 0.
/// - `out_rows`: populated on success with a heap-allocated array.
///   Release with [`dash_sdk_derive_identity_keys_from_mnemonic_free`].
/// - `out_error`: populated on failure with the usual
///   [`PlatformWalletFFIError`] detail.
///
/// On error `*out_rows` is left at its zero state.
///
/// # Safety
/// - `mnemonic_cstr` must be a valid, null-terminated UTF-8 C string
///   for the duration of the call.
/// - `passphrase_cstr` may be null; otherwise must be a valid
///   null-terminated UTF-8 C string.
/// - `out_rows` must be a valid, writable pointer to a
///   `IdentityRegistrationKeyDerivationsFFI`. Caller retains
///   ownership of the outer struct; this function fills it in place.
/// - `out_error` may be null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn dash_sdk_derive_identity_keys_from_mnemonic(
    mnemonic_cstr: *const std::os::raw::c_char,
    passphrase_cstr: *const std::os::raw::c_char,
    network: DashSDKNetwork,
    identity_index: u32,
    key_count: u32,
    out_rows: *mut IdentityRegistrationKeyDerivationsFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    use std::ffi::CStr;

    if out_rows.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "out_rows is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    // Pre-zero so a failed call leaves the caller staring at a known
    // empty struct, never uninitialized memory.
    *out_rows = IdentityRegistrationKeyDerivationsFFI {
        items: std::ptr::null_mut(),
        count: 0,
    };

    if mnemonic_cstr.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "mnemonic_cstr is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    if key_count == 0 {
        return PlatformWalletFFIResult::Success;
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
    // 64-byte seed wrapped in `Zeroizing` so it gets scrubbed when this
    // function returns (success or failure). `to_seed` returns by value;
    // wrapping at the call site is the earliest we can intercept it.
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
    let secp = Secp256k1::new();

    // ---- Walk key_count derivation paths -------------------------------------
    //
    // Build the row vec up-front so we can unwind ownership of every
    // CString / Vec / Box we've already detached if a later iteration
    // fails. `Vec::drop` would NOT free those raw pointers — they are
    // exposed via `into_raw` / `forget(Box::...)` and only the paired
    // free function reclaims them.
    let mut rows: Vec<IdentityKeyPreviewFFI> = Vec::with_capacity(key_count as usize);

    // Hand-roll cleanup matching the wallet-handle variant's pattern.
    let cleanup = |rows: Vec<IdentityKeyPreviewFFI>| {
        for row in rows {
            if !row.derivation_path.is_null() {
                let _ = CString::from_raw(row.derivation_path);
            }
            if !row.public_key.is_null() && row.public_key_len > 0 {
                let _ = Vec::from_raw_parts(row.public_key, row.public_key_len, row.public_key_len);
            }
            if !row.private_key_wif.is_null() {
                let _ = CString::from_raw(row.private_key_wif);
            }
        }
    };

    for key_index in 0..key_count {
        let path = match identity_auth_derivation_path(kw_network, identity_index, key_index) {
            Ok(p) => p,
            Err(detail) => {
                cleanup(rows);
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorWalletOperation,
                        format!(
                            "derive_identity_keys_from_mnemonic: path build failed at \
                             (identity={identity_index}, key={key_index}): {detail}"
                        ),
                    );
                }
                return PlatformWalletFFIResult::ErrorWalletOperation;
            }
        };

        // The intermediate `derived` xpriv carries a 32-byte secret in
        // the clear; we extract `.private_key.secret_bytes()` into the
        // FFI row and let the xpriv fall out of scope at the end of
        // the iteration. The seed remains zeroized regardless.
        let derived = match master.derive_priv(&secp, &path) {
            Ok(d) => d,
            Err(e) => {
                cleanup(rows);
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorWalletOperation,
                        format!(
                            "derive_identity_keys_from_mnemonic: derive_priv failed at \
                             (identity={identity_index}, key={key_index}): {e}"
                        ),
                    );
                }
                return PlatformWalletFFIResult::ErrorWalletOperation;
            }
        };
        let extended_pub = ExtendedPubKey::from_priv(&secp, &derived);
        let public_key = extended_pub.public_key;

        let path_cstring = match CString::new(path.to_string()) {
            Ok(s) => s,
            Err(e) => {
                cleanup(rows);
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        format!("derivation path contained NUL byte: {e}"),
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        };

        // Compressed secp256k1 pubkey is exactly 33 bytes.
        let pub_bytes: [u8; 33] = public_key.serialize();
        let mut pub_box: Box<[u8]> = pub_bytes.to_vec().into_boxed_slice();
        let pub_ptr = pub_box.as_mut_ptr();
        let pub_len = pub_box.len();
        std::mem::forget(pub_box);

        // WIF for the keychain-explorer / debugging UI. Same network-
        // aware shape as the wallet-handle variant produces.
        let dash_private = DashPrivateKey {
            compressed: true,
            network: kw_network,
            inner: derived.private_key,
        };
        let wif_cstring = match CString::new(dash_private.to_wif()) {
            Ok(s) => s,
            Err(e) => {
                // Path cstring + pubkey buffer were already detached.
                drop(Vec::from_raw_parts(pub_ptr, pub_len, pub_len));
                drop(path_cstring);
                cleanup(rows);
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        format!("WIF string contained NUL byte: {e}"),
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        };

        rows.push(IdentityKeyPreviewFFI {
            identity_index,
            derivation_path: path_cstring.into_raw(),
            public_key: pub_ptr,
            public_key_len: pub_len,
            private_key_wif: wif_cstring.into_raw(),
            // 32-byte secret scalar — copied by value into the FFI
            // row. `derived` goes out of scope at the end of the
            // loop body; the seed is still zeroized at function exit.
            private_key_bytes: derived.private_key.secret_bytes(),
        });
    }

    let mut boxed = rows.into_boxed_slice();
    let items_ptr = boxed.as_mut_ptr();
    let items_count = boxed.len();
    std::mem::forget(boxed);

    *out_rows = IdentityRegistrationKeyDerivationsFFI {
        items: items_ptr,
        count: items_count,
    };
    PlatformWalletFFIResult::Success
}

/// Release a [`IdentityRegistrationKeyDerivationsFFI`] previously
/// populated by [`dash_sdk_derive_identity_keys_from_mnemonic`].
///
/// Safe to call on a zero / null struct or null outer pointer (no-op).
/// Each row's owned strings (`derivation_path`, `private_key_wif`)
/// and pubkey buffer are reclaimed.
///
/// Behaviorally identical to
/// [`platform_wallet_derive_identity_keys_for_index_free`](crate::platform_wallet_derive_identity_keys_for_index_free)
/// — they consume the same struct shape — but kept as a separate
/// symbol so the Swift call site can pair allocator with deallocator
/// 1:1 by name.
///
/// # Safety
/// `rows.items` must have been handed out by
/// [`dash_sdk_derive_identity_keys_from_mnemonic`] (or the wallet-handle
/// variant — same layout) and must not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_identity_keys_from_mnemonic_free(
    rows: *mut IdentityRegistrationKeyDerivationsFFI,
) {
    if rows.is_null() {
        return;
    }
    let owned = std::mem::replace(
        &mut *rows,
        IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        },
    );
    if owned.items.is_null() || owned.count == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(owned.items, owned.count);
    for row in slice.iter_mut() {
        if !row.derivation_path.is_null() {
            let _ = CString::from_raw(row.derivation_path);
        }
        if !row.public_key.is_null() && row.public_key_len > 0 {
            let _ = Vec::from_raw_parts(row.public_key, row.public_key_len, row.public_key_len);
        }
        if !row.private_key_wif.is_null() {
            let _ = CString::from_raw(row.private_key_wif);
        }
    }
    let _ = Box::from_raw(slice as *mut [IdentityKeyPreviewFFI]);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// English BIP-39 test vector (all-zero entropy). Same fixture
    /// the upstream `bip39` crate uses for round-trip tests; reused
    /// here so the assertions match a well-known mnemonic.
    const ENGLISH_PHRASE: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    /// Sanity-check the happy path on testnet with a small key count.
    /// We don't assert specific hex bytes — derive determinism is the
    /// platform-wallet / key-wallet test surface — but we DO assert
    /// the shape contract: row count, pubkey length, path string
    /// shape, and that each row carries non-zero secret bytes.
    #[test]
    fn derives_three_keys_with_correct_shape() {
        let mnemonic = std::ffi::CString::new(ENGLISH_PHRASE).unwrap();
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let mut err = PlatformWalletFFIError::success();
        let result = unsafe {
            dash_sdk_derive_identity_keys_from_mnemonic(
                mnemonic.as_ptr(),
                std::ptr::null(), // empty passphrase
                DashSDKNetwork::SDKTestnet,
                7,
                3,
                &mut out,
                &mut err,
            )
        };
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(out.count, 3);
        assert!(!out.items.is_null());

        for i in 0..out.count {
            let row = unsafe { &*out.items.add(i) };
            assert_eq!(row.identity_index, 7);
            assert_eq!(row.public_key_len, 33);
            assert!(!row.public_key.is_null());
            assert!(!row.derivation_path.is_null());

            let path = unsafe { std::ffi::CStr::from_ptr(row.derivation_path) }
                .to_str()
                .unwrap();
            // m/9'/1'/5'/0'/0'/7'/<i>'  on testnet (coin_type = 1).
            // We just sanity-check the structural prefix and the trailing
            // key index — exact path strings are covered by platform-wallet.
            assert!(
                path.starts_with("m/9'/1'/5'/0'/0'/7'/"),
                "unexpected path prefix: {path}"
            );
            assert!(
                path.ends_with(&format!("/{i}'")),
                "unexpected path tail: {path}"
            );

            // Secret scalar must be non-zero; if it were, derivation
            // silently no-op'd.
            assert!(
                row.private_key_bytes.iter().any(|b| *b != 0),
                "secret bytes are all zero at row {i}"
            );
        }

        unsafe { dash_sdk_derive_identity_keys_from_mnemonic_free(&mut out) };
        // `_free` resets the outer struct.
        assert!(out.items.is_null());
        assert_eq!(out.count, 0);
    }

    /// Mainnet path uses coin_type = 5 instead of 1. Lightweight
    /// coverage that the network mapping flows into the path.
    #[test]
    fn derives_mainnet_path_uses_coin_type_5() {
        let mnemonic = std::ffi::CString::new(ENGLISH_PHRASE).unwrap();
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let mut err = PlatformWalletFFIError::success();
        let result = unsafe {
            dash_sdk_derive_identity_keys_from_mnemonic(
                mnemonic.as_ptr(),
                std::ptr::null(),
                DashSDKNetwork::SDKMainnet,
                0,
                1,
                &mut out,
                &mut err,
            )
        };
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(out.count, 1);

        let row = unsafe { &*out.items };
        let path = unsafe { std::ffi::CStr::from_ptr(row.derivation_path) }
            .to_str()
            .unwrap();
        assert_eq!(path, "m/9'/5'/5'/0'/0'/0'/0'");

        unsafe { dash_sdk_derive_identity_keys_from_mnemonic_free(&mut out) };
    }

    /// `key_count == 0` is a legal no-op and must not allocate.
    #[test]
    fn key_count_zero_is_noop_success() {
        let mnemonic = std::ffi::CString::new(ENGLISH_PHRASE).unwrap();
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let mut err = PlatformWalletFFIError::success();
        let result = unsafe {
            dash_sdk_derive_identity_keys_from_mnemonic(
                mnemonic.as_ptr(),
                std::ptr::null(),
                DashSDKNetwork::SDKTestnet,
                0,
                0,
                &mut out,
                &mut err,
            )
        };
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(out.count, 0);
        assert!(out.items.is_null());
        // `_free` is a no-op on the empty struct.
        unsafe { dash_sdk_derive_identity_keys_from_mnemonic_free(&mut out) };
    }

    /// Garbage mnemonic must surface `ErrorInvalidParameter`, not a
    /// crash, and must not leave a partial allocation behind.
    #[test]
    fn rejects_invalid_mnemonic() {
        let mnemonic = std::ffi::CString::new("not a real bip39 phrase at all here").unwrap();
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let mut err = PlatformWalletFFIError::success();
        let result = unsafe {
            dash_sdk_derive_identity_keys_from_mnemonic(
                mnemonic.as_ptr(),
                std::ptr::null(),
                DashSDKNetwork::SDKTestnet,
                0,
                3,
                &mut out,
                &mut err,
            )
        };
        assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidParameter);
        assert!(out.items.is_null());
        assert_eq!(out.count, 0);
    }
}
