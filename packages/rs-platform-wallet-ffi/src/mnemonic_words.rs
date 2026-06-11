//! BIP-39 recover-flow word/phrase helpers — **thin FFI surface**.
//!
//! The recover-flow logic (CJK ideographic auto-split + punctuation cleanup,
//! the 7-language DashSync bundled-language union, BCP-47 language mapping)
//! lives in the non-FFI sibling crate [`platform_wallet::mnemonic`]. These
//! `extern "C"` entry points only marshal C strings to/from Rust and delegate.
//! `normalizePhrase` is a raw BIP-39 primitive with no platform policy, so it
//! calls `key_wallet::Mnemonic::normalize_phrase` directly (routing it through
//! a `platform_wallet` passthrough would just re-add a pointless wrapper).
//!
//! DashSync `DSBIP39Mnemonic` parity map:
//!   - `wordIsValid:`     -> [`platform_wallet_mnemonic_word_is_valid`]
//!   - `wordIsLocal:`     -> generalized: [`platform_wallet_mnemonic_word_is_in_language`]
//!   - `normalizePhrase:` -> [`platform_wallet_mnemonic_normalize_phrase`]
//!   - `cleanupPhrase:`   -> [`platform_wallet_mnemonic_cleanup_phrase`]

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::error::PlatformWalletFFIResult;
use crate::{check_ptr, unwrap_result_or_return};

/// `true` if `word` is a BIP-39 word in any DashSync-bundled language
/// (`wordIsValid:`). NULL / invalid-UTF-8 input returns `false` (the recover
/// UI treats unknown words as "incorrect", matching DashSync's outcome).
///
/// # Safety
/// `word` must be NULL or a valid null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_mnemonic_word_is_valid(word: *const c_char) -> bool {
    if word.is_null() {
        return false;
    }
    match CStr::from_ptr(word).to_str() {
        Ok(w) => platform_wallet::mnemonic::word_in_any_list(w),
        Err(_) => false,
    }
}

/// `true` if `word` is a BIP-39 word in `language` (a BCP-47-ish code such as
/// `"en"`, `"ja"`, or `"zh-hans"`). NULL / invalid-UTF-8 / unrecognized-language
/// input returns `false`. Replaces the former English-hardcoded
/// `platform_wallet_mnemonic_word_is_local`: which language is "local" is an
/// app-level choice, so the caller passes it explicitly.
///
/// # Safety
/// `word` and `language` must each be NULL or a valid null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_mnemonic_word_is_in_language(
    word: *const c_char,
    language: *const c_char,
) -> bool {
    if word.is_null() || language.is_null() {
        return false;
    }
    let w = match CStr::from_ptr(word).to_str() {
        Ok(w) => w,
        Err(_) => return false,
    };
    let code = match CStr::from_ptr(language).to_str() {
        Ok(c) => c,
        Err(_) => return false,
    };
    platform_wallet::mnemonic::word_in_language(w, code)
}

/// NFKD + lowercase + whitespace-collapse (`normalizePhrase:`). Raw key-wallet
/// primitive (no platform policy). On success the caller owns `*out_string` and
/// must free it via [`crate::xpub_render::platform_wallet_free_string`].
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

/// Minimal cleanup + CJK auto-split (`cleanupPhrase:`). On success the caller
/// owns `*out_string` and must free it via
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
    let cleaned = platform_wallet::mnemonic::cleanup_phrase(p);
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
    /// `into_raw`/`free_string` ownership handoff, and the null-pointer error
    /// codes. The recover-flow algorithm is tested in `platform_wallet::mnemonic`.
    #[test]
    fn ffi_roundtrip() {
        use std::ffi::CString;

        // bool fns
        let valid = CString::new("abandon").unwrap();
        let invalid = CString::new("notaword").unwrap();
        let en = CString::new("en").unwrap();
        let ja = CString::new("ja").unwrap();
        unsafe {
            assert!(platform_wallet_mnemonic_word_is_valid(valid.as_ptr()));
            assert!(!platform_wallet_mnemonic_word_is_valid(invalid.as_ptr()));
            assert!(!platform_wallet_mnemonic_word_is_valid(std::ptr::null()));
            // word_is_in_language: English word is in "en", not in "ja"; NULL
            // word / NULL language / unrecognized language all return false.
            assert!(platform_wallet_mnemonic_word_is_in_language(
                valid.as_ptr(),
                en.as_ptr()
            ));
            assert!(!platform_wallet_mnemonic_word_is_in_language(
                valid.as_ptr(),
                ja.as_ptr()
            ));
            assert!(!platform_wallet_mnemonic_word_is_in_language(
                std::ptr::null(),
                en.as_ptr()
            ));
            assert!(!platform_wallet_mnemonic_word_is_in_language(
                valid.as_ptr(),
                std::ptr::null()
            ));
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

        // NULL out-pointer -> error
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
