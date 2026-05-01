//! Stateless derivation helpers.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::str::FromStr;

use dashcore::secp256k1::Secp256k1;
use key_wallet::bip32::{DerivationPath, ExtendedPrivKey};
use key_wallet::mnemonic::{Language, Mnemonic};
use zeroize::Zeroizing;

use crate::error::*;
use crate::types::{FFINetwork, Network};
use crate::{check_ptr, unwrap_result_or_return};

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
/// * `network` — Drives the xpriv version bytes; the derived secret
///    key itself is network-independent but the master key's chain
///    code derivation uses the HD seed directly.
/// * `path_utf8` — UTF-8 null-terminated BIP-32 path string, e.g.
///    `m/9'/5'/0'/0'/0'`. Parsed via [`DerivationPath::from_str`].
/// * `out_secret_key` — 32 byte buffer receiving the private key.
///    Caller must ensure the buffer is 32 bytes.
/// * `out_chain_code` — 32 byte buffer receiving the chain code.
/// * `out_public_key` — 33 byte buffer receiving the compressed
///    secp256k1 pubkey. Pass `null` to skip.
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
    network: FFINetwork,
    path_utf8: *const c_char,
    out_secret_key: *mut u8,
    out_chain_code: *mut u8,
    out_public_key: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(mnemonic);
    check_ptr!(path_utf8);
    check_ptr!(out_secret_key);
    check_ptr!(out_chain_code);

    let mnemonic_str = unwrap_result_or_return!(CStr::from_ptr(mnemonic).to_str());
    let passphrase_str: &str = if passphrase.is_null() {
        ""
    } else {
        unwrap_result_or_return!(CStr::from_ptr(passphrase).to_str())
    };
    let path_str = unwrap_result_or_return!(CStr::from_ptr(path_utf8).to_str());

    let network: Network = network.into();

    let path = unwrap_result_or_return!(DerivationPath::from_str(path_str));

    let mnemonic_obj = unwrap_result_or_return!(parse_mnemonic_any_language(mnemonic_str));

    let seed = Zeroizing::new(mnemonic_obj.to_seed(passphrase_str));

    let master = unwrap_result_or_return!(ExtendedPrivKey::new_master(network, &*seed));

    let secp = Secp256k1::new();
    let derived = unwrap_result_or_return!(master.derive_priv(&secp, &path));

    let secret = Zeroizing::new(derived.private_key.secret_bytes());
    std::ptr::copy_nonoverlapping(secret.as_ptr(), out_secret_key, 32);

    std::ptr::copy_nonoverlapping(derived.chain_code.as_ref().as_ptr(), out_chain_code, 32);

    if !out_public_key.is_null() {
        let pubkey_bytes = derived.private_key.public_key(&secp).serialize();
        std::ptr::copy_nonoverlapping(pubkey_bytes.as_ptr(), out_public_key, 33);
    }

    PlatformWalletFFIResult::ok()
}
