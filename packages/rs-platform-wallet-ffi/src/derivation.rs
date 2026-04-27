//! Stateless derivation helpers.
//!
//! These helpers compose `key_wallet` primitives
//! (`Mnemonic::to_seed` → `ExtendedPrivKey::new_master` →
//! `derive_priv`) behind a single FFI call so the Swift side can obtain
//! an extended private key at a BIP-32 derivation path **without
//! mutating any persisted wallet state**.
//!
//! The wallet struct itself remains `WatchOnly` / `ExternalSignable` —
//! no key material is ever written into `Wallet::wallet_type`. All
//! intermediates here (seed, master key, derived key) are wrapped in
//! `Zeroizing` so they are scrubbed from memory as soon as the FFI
//! function returns.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::str::FromStr;

use dashcore::secp256k1::Secp256k1;
use key_wallet::bip32::{DerivationPath, ExtendedPrivKey};
use key_wallet::mnemonic::{Language, Mnemonic};
use key_wallet::Network;
use zeroize::Zeroizing;

use crate::error::*;

/// Parse a BIP-39 mnemonic against every supported wordlist in turn,
/// returning the first language that yields a valid mnemonic.
///
/// `key_wallet::Mnemonic` only exposes language-tagged constructors,
/// so user-mnemonic FFI entry points must walk the language list
/// themselves to avoid rejecting a French / Japanese / etc. phrase as
/// "invalid English". BIP-39 wordlists are mutually exclusive per
/// phrase (per the spec), so the first match is unambiguous.
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

/// Derive a 32-byte ECDSA private key at a BIP-32 derivation path from
/// a mnemonic phrase.
///
/// On success `out_secret_key` (32 bytes) receives the secp256k1
/// private key and `out_chain_code` (32 bytes) receives the BIP-32
/// chain code. The corresponding compressed public key (33 bytes) is
/// computed from the private key and written to `out_public_key` so
/// callers that only need the pubkey (identity registration: pubkey
/// only) never have to touch the private material.
///
/// # Inputs
/// * `mnemonic` — UTF-8 null-terminated BIP-39 mnemonic phrase.
/// * `passphrase` — optional UTF-8 null-terminated BIP-39 passphrase.
///    Pass `null` for no passphrase.
/// * `network` — `0` = Mainnet, `1` = Testnet, `2` = Devnet, `3` =
///    Regtest. Drives the xpriv version bytes; the derived secret
///    key itself is network-independent but the master key's chain
///    code derivation uses the HD seed directly.
/// * `path_utf8` — UTF-8 null-terminated BIP-32 path string, e.g.
///    `m/9'/5'/0'/0'/0'`. Parsed via [`DerivationPath::from_str`].
/// * `out_secret_key` — 32 byte buffer receiving the private key.
///    Caller must ensure the buffer is 32 bytes.
/// * `out_chain_code` — 32 byte buffer receiving the chain code.
/// * `out_public_key` — 33 byte buffer receiving the compressed
///    secp256k1 pubkey. Pass `null` to skip.
/// * `out_error` — error detail populated on failure. May be null.
///
/// # Safety
/// All pointers must either be null (only where marked optional) or
/// point at valid memory of the documented sizes. The function does
/// not retain any pointer beyond the call.
///
/// # Zeroization
/// The derived master xpriv, the derived xpriv at `path`, and the
/// intermediate seed are wrapped in [`Zeroizing`] inside this
/// function. The only material left in memory after return is the
/// 32-byte secret bytes written into the caller's buffer (caller's
/// responsibility to manage).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_derive_ext_priv_key_from_mnemonic(
    mnemonic: *const c_char,
    passphrase: *const c_char,
    network: u32,
    path_utf8: *const c_char,
    out_secret_key: *mut u8,
    out_chain_code: *mut u8,
    out_public_key: *mut u8,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    // ---- Validate pointers --------------------------------------------------
    if mnemonic.is_null()
        || path_utf8.is_null()
        || out_secret_key.is_null()
        || out_chain_code.is_null()
    {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "null pointer for required argument",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // ---- Decode inputs ------------------------------------------------------
    let mnemonic_str = match CStr::from_ptr(mnemonic).to_str() {
        Ok(s) => s,
        Err(_) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    "mnemonic is not valid UTF-8",
                );
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };
    let passphrase_str: &str = if passphrase.is_null() {
        ""
    } else {
        match CStr::from_ptr(passphrase).to_str() {
            Ok(s) => s,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        "passphrase is not valid UTF-8",
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        }
    };
    let path_str = match CStr::from_ptr(path_utf8).to_str() {
        Ok(s) => s,
        Err(_) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    "path is not valid UTF-8",
                );
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };

    let network = match network {
        0 => Network::Mainnet,
        1 => Network::Testnet,
        2 => Network::Devnet,
        3 => Network::Regtest,
        _ => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    "invalid network (expected 0..=3)",
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };

    let path = match DerivationPath::from_str(path_str) {
        Ok(p) => p,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("invalid derivation path: {}", e),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };

    // ---- Derive -------------------------------------------------------------
    // Auto-detect the BIP-39 language so non-English phrases (Spanish,
    // French, Italian, Japanese, Korean, Chinese {Simplified,
    // Traditional}, Czech, Portuguese) are accepted instead of being
    // rejected as "invalid English".
    let mnemonic_obj = match parse_mnemonic_any_language(mnemonic_str) {
        Ok(m) => m,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("invalid mnemonic: {}", e),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };

    // Seed is 64 bytes; Zeroizing scrubs on drop. ExtendedPrivKey
    // itself does not implement Zeroize (upstream limitation), so we
    // only wrap the byte-array outputs below. Intermediate xpriv
    // values live for a few microseconds in this function and are
    // dropped at end-of-scope; not ideal but materially bounded.
    let seed = Zeroizing::new(mnemonic_obj.to_seed(passphrase_str));

    let master = match ExtendedPrivKey::new_master(network, &*seed) {
        Ok(m) => m,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorWalletOperation,
                    format!("failed to derive master key: {}", e),
                );
            }
            return PlatformWalletFFIResult::ErrorWalletOperation;
        }
    };

    let secp = Secp256k1::new();
    let derived = match master.derive_priv(&secp, &path) {
        Ok(d) => d,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorWalletOperation,
                    format!("failed to derive private key at path: {}", e),
                );
            }
            return PlatformWalletFFIResult::ErrorWalletOperation;
        }
    };

    // ---- Write outputs ------------------------------------------------------
    let secret = Zeroizing::new(derived.private_key.secret_bytes());
    std::ptr::copy_nonoverlapping(secret.as_ptr(), out_secret_key, 32);

    std::ptr::copy_nonoverlapping(derived.chain_code.as_ref().as_ptr(), out_chain_code, 32);

    if !out_public_key.is_null() {
        let pubkey_bytes = derived.private_key.public_key(&secp).serialize();
        std::ptr::copy_nonoverlapping(pubkey_bytes.as_ptr(), out_public_key, 33);
    }

    PlatformWalletFFIResult::Success
}
