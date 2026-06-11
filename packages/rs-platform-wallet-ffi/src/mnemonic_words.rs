//! BIP-39 recover-flow word/phrase helpers — **thin FFI surface**.
//!
//! All recover-flow logic lives in [`key_wallet::Mnemonic`] /
//! [`key_wallet::Language`] (rust-dashcore). These `extern "C"` entry points
//! only marshal C strings + the language enum to/from Rust and delegate:
//!   - `word_list(language)` -> [`key_wallet::Language::word_list`], joined
//!     with `\n` into one C string (BIP-39 words contain no whitespace);
//!   - `normalize_phrase`    -> [`key_wallet::Mnemonic::normalize_phrase`];
//!   - `cleanup_phrase`      -> [`key_wallet::Mnemonic::cleanup_phrase`].
//!
//! The recover UI composes its own word-validity checks — the any-language
//! "valid" union and the per-language "local" check — from `word_list`, so the
//! FFI no longer ships granular `word_is_valid` / `word_is_in_language`
//! entry points (they carried policy that belongs in the app).

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::error::PlatformWalletFFIResult;
use crate::{check_ptr, unwrap_result_or_return};

/// BIP-39 wordlist language — C-ABI mirror of [`key_wallet::Language`].
///
/// Discriminants match `key_wallet::Language`'s declaration order (and the
/// Swift `MnemonicLanguage` raw values) so the three enums stay numerically
/// aligned. Deliberately a distinct name from key-wallet-ffi's `FFILanguage`
/// (same 0–9 values): both headers are exported by the single `DashSDKFFI`
/// umbrella module, so reusing the name would be a C enumerator redefinition.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FFIMnemonicLanguage {
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

impl From<FFIMnemonicLanguage> for key_wallet::Language {
    fn from(language: FFIMnemonicLanguage) -> Self {
        match language {
            FFIMnemonicLanguage::English => key_wallet::Language::English,
            FFIMnemonicLanguage::ChineseSimplified => key_wallet::Language::ChineseSimplified,
            FFIMnemonicLanguage::ChineseTraditional => key_wallet::Language::ChineseTraditional,
            FFIMnemonicLanguage::Czech => key_wallet::Language::Czech,
            FFIMnemonicLanguage::French => key_wallet::Language::French,
            FFIMnemonicLanguage::Italian => key_wallet::Language::Italian,
            FFIMnemonicLanguage::Japanese => key_wallet::Language::Japanese,
            FFIMnemonicLanguage::Korean => key_wallet::Language::Korean,
            FFIMnemonicLanguage::Portuguese => key_wallet::Language::Portuguese,
            FFIMnemonicLanguage::Spanish => key_wallet::Language::Spanish,
        }
    }
}

/// Raw BIP-39 wordlist (2048 words) for `language`, joined into one
/// newline-separated C string. BIP-39 words never contain whitespace, so `\n`
/// is an unambiguous separator; the caller splits on `\n`. On success the
/// caller owns `*out_string` and must free it via
/// [`crate::xpub_render::platform_wallet_free_string`].
///
/// # Safety
/// `out_string` must point to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_mnemonic_word_list(
    language: FFIMnemonicLanguage,
    out_string: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_string);
    *out_string = std::ptr::null_mut();

    let language: key_wallet::Language = language.into();
    let joined = language.word_list().join("\n");
    let c = unwrap_result_or_return!(CString::new(joined));
    *out_string = c.into_raw();
    PlatformWalletFFIResult::ok()
}

/// NFKD + lowercase + whitespace-collapse (`normalizePhrase:`). Raw key-wallet
/// primitive (no platform policy). On success the caller owns `*out_string`
/// and must free it via [`crate::xpub_render::platform_wallet_free_string`].
///
/// # Safety
/// `phrase` must point to a valid null-terminated UTF-8 C string; `out_string`
/// must point to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_mnemonic_normalize_phrase(
    phrase: *const c_char,
    out_string: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(phrase);
    check_ptr!(out_string);
    *out_string = std::ptr::null_mut();

    let p = unwrap_result_or_return!(CStr::from_ptr(phrase).to_str());
    let normalized = key_wallet::Mnemonic::normalize_phrase(p);
    let c = unwrap_result_or_return!(CString::new(normalized));
    *out_string = c.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Minimal cleanup + CJK auto-split (`cleanupPhrase:`). Raw key-wallet
/// primitive. On success the caller owns `*out_string` and must free it via
/// [`crate::xpub_render::platform_wallet_free_string`].
///
/// # Safety
/// `phrase` must point to a valid null-terminated UTF-8 C string; `out_string`
/// must point to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_mnemonic_cleanup_phrase(
    phrase: *const c_char,
    out_string: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(phrase);
    check_ptr!(out_string);
    *out_string = std::ptr::null_mut();

    let p = unwrap_result_or_return!(CStr::from_ptr(phrase).to_str());
    let cleaned = key_wallet::Mnemonic::cleanup_phrase(p);
    let c = unwrap_result_or_return!(CString::new(cleaned));
    *out_string = c.into_raw();
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PlatformWalletFFIResultCode;
    use crate::xpub_render::platform_wallet_free_string;

    /// Exercises the `extern "C"` boundary itself — CString in/out, the
    /// `into_raw`/`free_string` ownership handoff, the language-enum
    /// conversion, and the null-pointer error codes. The recover-flow
    /// algorithm (cleanup/normalize/word_list) is tested in
    /// `key_wallet::Mnemonic`.
    #[test]
    fn ffi_roundtrip() {
        use std::ffi::CString;

        // word_list: English -> 2048 \n-joined words including "abandon"/"zoo".
        let mut wl: *mut c_char = std::ptr::null_mut();
        unsafe {
            let r = platform_wallet_mnemonic_word_list(FFIMnemonicLanguage::English, &mut wl);
            assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
            assert!(!wl.is_null());
            let s = CStr::from_ptr(wl).to_str().unwrap().to_owned();
            let words: Vec<&str> = s.split('\n').collect();
            assert_eq!(words.len(), 2048);
            assert_eq!(words[0], "abandon");
            assert_eq!(words[2047], "zoo");
            platform_wallet_free_string(wl);
        }

        // word_list: a different language yields a different list (Japanese);
        // confirms the enum conversion actually selects the wordlist.
        let mut wl_ja: *mut c_char = std::ptr::null_mut();
        unsafe {
            let r = platform_wallet_mnemonic_word_list(FFIMnemonicLanguage::Japanese, &mut wl_ja);
            assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
            let s = CStr::from_ptr(wl_ja).to_str().unwrap().to_owned();
            assert_eq!(s.split('\n').count(), 2048);
            assert!(!s.split('\n').any(|w| w == "abandon"));
            platform_wallet_free_string(wl_ja);
        }

        // word_list NULL out-pointer -> error
        unsafe {
            let r = platform_wallet_mnemonic_word_list(
                FFIMnemonicLanguage::English,
                std::ptr::null_mut(),
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }

        // string fn: normalize
        let input = CString::new("  ABANDON about ").unwrap();
        let mut out: *mut c_char = std::ptr::null_mut();
        unsafe {
            let r = platform_wallet_mnemonic_normalize_phrase(input.as_ptr(), &mut out);
            assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
            assert!(!out.is_null());
            let s = CStr::from_ptr(out).to_str().unwrap().to_owned();
            assert_eq!(s, "abandon about");
            platform_wallet_free_string(out);
        }

        // NULL phrase -> error
        let mut out2: *mut c_char = std::ptr::null_mut();
        unsafe {
            let r = platform_wallet_mnemonic_normalize_phrase(std::ptr::null(), &mut out2);
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }

        // string fn: cleanup — exercises the cleanup into_raw/free path too
        let dirty = CString::new(
            "abandon, abandon. abandon abandon abandon abandon abandon abandon abandon abandon abandon about!",
        )
        .unwrap();
        let mut cout: *mut c_char = std::ptr::null_mut();
        unsafe {
            let r = platform_wallet_mnemonic_cleanup_phrase(dirty.as_ptr(), &mut cout);
            assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
            assert!(!cout.is_null());
            let s = CStr::from_ptr(cout).to_str().unwrap().to_owned();
            assert!(!s.contains(','));
            assert!(!s.contains('!'));
            platform_wallet_free_string(cout);
        }

        // cleanup NULL phrase -> error
        let mut cout2: *mut c_char = std::ptr::null_mut();
        unsafe {
            let r = platform_wallet_mnemonic_cleanup_phrase(std::ptr::null(), &mut cout2);
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
    }
}
