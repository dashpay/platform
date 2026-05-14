//! BIP-39 mnemonic and BIP-32 derivation primitives.
//!
//! Thin C-ABI wrappers over the `key-wallet` crate — the surface that
//! `key-wallet-ffi` used to provide before it was dropped from this repo.

use crate::error::*;
use crate::{check_ptr, unwrap_result_or_return};
use dash_network::ffi::FFINetwork;
use key_wallet::mnemonic::{Language, Mnemonic};
use std::os::raw::c_char;

/// BIP-39 wordlist language. Mirrors `key_wallet::mnemonic::Language`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum FFILanguage {
    English = 0,
    ChineseSimplified = 1,
    ChineseTraditional = 2,
    Czech = 3,
    French = 4,
    Italian = 5,
    Japanese = 6,
    Korean = 7,
    Portuguese = 8,
    Spanish = 9,
}

impl From<FFILanguage> for Language {
    fn from(language: FFILanguage) -> Self {
        match language {
            FFILanguage::English => Language::English,
            FFILanguage::ChineseSimplified => Language::ChineseSimplified,
            FFILanguage::ChineseTraditional => Language::ChineseTraditional,
            FFILanguage::Czech => Language::Czech,
            FFILanguage::French => Language::French,
            FFILanguage::Italian => Language::Italian,
            FFILanguage::Japanese => Language::Japanese,
            FFILanguage::Korean => Language::Korean,
            FFILanguage::Portuguese => Language::Portuguese,
            FFILanguage::Spanish => Language::Spanish,
        }
    }
}

/// Generate a fresh BIP-39 mnemonic of `word_count` words (12, 15, 18,
/// 21, or 24) in `language`. Writes a heap C string into `out_mnemonic`
/// (caller frees via `platform_wallet_string_free`).
///
/// # Safety
/// `out_mnemonic` must be a valid writable `*mut c_char` location.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_generate_mnemonic(
    word_count: u32,
    language: FFILanguage,
    out_mnemonic: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_mnemonic);
    let phrase = unwrap_result_or_return!(Mnemonic::generate(word_count as usize, language.into()));
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(phrase.to_string()));
    *out_mnemonic = c_str.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Build the DIP-9 identity-authentication path
/// `m/9'/<coin>'/5'/0'/identity_index'/key_index'`. Writes a heap C
/// string into `out_path` (caller frees via `platform_wallet_string_free`).
/// Returns 0 on success, -1 on failure.
///
/// # Safety
/// `out_path` must be a valid writable `*mut c_char` location.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_derive_identity_authentication_path(
    network: FFINetwork,
    identity_index: u32,
    key_index: u32,
    out_path: *mut *mut c_char,
) -> i32 {
    if out_path.is_null() {
        return -1;
    }
    use key_wallet::bip32::{DerivationPath, KeyDerivationType};
    let net: key_wallet::Network = network.into();
    let derivation = DerivationPath::identity_authentication_path(
        net,
        KeyDerivationType::ECDSA,
        identity_index,
        key_index,
    );
    let path_str = format!("{}", derivation);
    let Ok(c_str) = std::ffi::CString::new(path_str) else {
        return -1;
    };
    *out_path = c_str.into_raw();
    0
}
