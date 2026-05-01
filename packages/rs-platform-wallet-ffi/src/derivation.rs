//! Stateless derivation helpers.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::str::FromStr;

use dashcore::secp256k1::Secp256k1;
use key_wallet::bip32::{DerivationPath, ExtendedPrivKey};
use key_wallet::mnemonic::{Language, Mnemonic};
use key_wallet::Network;
use zeroize::Zeroizing;

use crate::error::*;
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
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_derive_ext_priv_key_from_mnemonic(
    mnemonic: *const c_char,
    passphrase: *const c_char,
    network: u32,
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

    let network = match network {
        0 => Network::Mainnet,
        1 => Network::Testnet,
        2 => Network::Devnet,
        3 => Network::Regtest,
        _ => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "invalid network (expected 0..=3)",
            );
        }
    };

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
