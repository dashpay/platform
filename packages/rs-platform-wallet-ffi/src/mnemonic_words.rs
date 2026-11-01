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
use std::ptr;

use zeroize::{Zeroize, Zeroizing};

use crate::address_private_key::secret_string_into_raw;
use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::{check_ptr, unwrap_result_or_return};
use platform_wallet::parse_mnemonic_any_language;

/// BIP-39 wordlist language — C-ABI mirror of [`key_wallet::Language`].
///
/// Discriminants match `key_wallet::Language`'s declaration order (and the
/// Swift `MnemonicLanguage` raw values) so the three enums stay numerically
/// aligned.
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

impl TryFrom<u32> for FFIMnemonicLanguage {
    type Error = PlatformWalletFFIResult;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::English,
            1 => Self::ChineseSimplified,
            2 => Self::ChineseTraditional,
            3 => Self::Czech,
            4 => Self::French,
            5 => Self::Italian,
            6 => Self::Japanese,
            7 => Self::Korean,
            8 => Self::Portuguese,
            9 => Self::Spanish,
            _ => {
                return Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    format!("unknown mnemonic language {value}"),
                ))
            }
        })
    }
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
/// `language` is an [`FFIMnemonicLanguage`] value, taken as a plain integer so
/// an unknown value fails with `ErrorInvalidParameter` instead of being
/// undefined behaviour.
///
/// # Safety
/// `out_string` must point to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_mnemonic_word_list(
    language: u32,
    out_string: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_string);
    *out_string = std::ptr::null_mut();

    let language: key_wallet::Language =
        unwrap_result_or_return!(FFIMnemonicLanguage::try_from(language)).into();
    let joined = language.word_list().join("\n");
    let c = unwrap_result_or_return!(CString::new(joined));
    *out_string = c.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Generates a random BIP-39 mnemonic of `word_count` words in `language`.
///
/// key-wallet accepts 12, 15, 18, 21 or 24 words and refuses any other count
/// with `ErrorWalletOperation`. `language` is an [`FFIMnemonicLanguage`]
/// value, taken as a plain integer so an unknown value fails with
/// `ErrorInvalidParameter` instead of being undefined behaviour. On success
/// the caller owns `*out_mnemonic` and must free it via
/// [`platform_wallet_mnemonic_free`].
///
/// # Safety
/// `out_mnemonic` must point to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_mnemonic_generate(
    word_count: u32,
    language: u32,
    out_mnemonic: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_mnemonic);
    *out_mnemonic = ptr::null_mut();

    let language = unwrap_result_or_return!(FFIMnemonicLanguage::try_from(language));
    let mnemonic = unwrap_result_or_return!(key_wallet::Mnemonic::generate(
        word_count as usize,
        language.into()
    ));
    *out_mnemonic =
        unwrap_result_or_return!(secret_string_into_raw(Zeroizing::new(mnemonic.phrase())));
    PlatformWalletFFIResult::ok()
}

/// Checks that `phrase` is a valid BIP-39 mnemonic in any supported language.
///
/// Returns `Success` for a valid phrase and `ErrorInvalidParameter` for one in
/// no supported wordlist. A null `phrase` fails with `ErrorNullPointer`, and
/// one that is not UTF-8 with `ErrorUtf8Conversion`.
/// It accepts exactly the phrases the wallet manager accepts when it creates a
/// wallet.
///
/// # Safety
/// `phrase` must point to a valid null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_mnemonic_validate(
    phrase: *const c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(phrase);
    let phrase = unwrap_result_or_return!(CStr::from_ptr(phrase).to_str());
    unwrap_result_or_return!(parse_mnemonic_any_language(phrase));
    PlatformWalletFFIResult::ok()
}

/// Zeroizes and frees a mnemonic returned by
/// [`platform_wallet_mnemonic_generate`]. Safe to call with null.
///
/// # Safety
/// `mnemonic` must be null or a pointer returned by
/// [`platform_wallet_mnemonic_generate`] that has not been freed yet. The
/// caller must not have written to the buffer: the free finds its length by
/// the first NUL, so a moved terminator frees the wrong size and leaves the
/// rest of the phrase unwiped.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_mnemonic_free(mnemonic: *mut c_char) {
    if mnemonic.is_null() {
        return;
    }
    CString::from_raw(mnemonic).into_bytes_with_nul().zeroize();
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
    use crate::xpub_render::platform_wallet_free_string;
    use platform_wallet::test_support::{
        TEST_FRENCH_MNEMONIC, TEST_INVALID_MNEMONIC, TEST_PLATFORM_WALLET_MNEMONIC,
    };
    use std::ptr::NonNull;

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
            let r =
                platform_wallet_mnemonic_word_list(FFIMnemonicLanguage::English as u32, &mut wl);
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
            let r = platform_wallet_mnemonic_word_list(
                FFIMnemonicLanguage::Japanese as u32,
                &mut wl_ja,
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
            let s = CStr::from_ptr(wl_ja).to_str().unwrap().to_owned();
            assert_eq!(s.split('\n').count(), 2048);
            assert!(!s.split('\n').any(|w| w == "abandon"));
            platform_wallet_free_string(wl_ja);
        }

        // word_list NULL out-pointer -> error
        unsafe {
            let r = platform_wallet_mnemonic_word_list(
                FFIMnemonicLanguage::English as u32,
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

        // generate: a non-English phrase that parses back in its language.
        let mut phrase: *mut c_char = ptr::null_mut();
        unsafe {
            let r = platform_wallet_mnemonic_generate(
                15,
                FFIMnemonicLanguage::French as u32,
                &mut phrase,
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
            let s = CStr::from_ptr(phrase).to_str().unwrap().to_owned();
            let parsed = key_wallet::Mnemonic::from_phrase(&s).unwrap();
            assert_eq!(parsed.word_count(), 15);
            assert_eq!(parsed.language(), key_wallet::Language::French);
            platform_wallet_mnemonic_free(phrase);
        }

        // generate: invalid word count -> error with a nulled out-pointer
        let mut bad: *mut c_char = NonNull::dangling().as_ptr();
        unsafe {
            let r = platform_wallet_mnemonic_generate(
                13,
                FFIMnemonicLanguage::English as u32,
                &mut bad,
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorWalletOperation);
            assert!(bad.is_null());
        }

        // an unknown language is refused before it becomes an enum
        let mut bad: *mut c_char = NonNull::dangling().as_ptr();
        unsafe {
            let r = platform_wallet_mnemonic_generate(12, 10, &mut bad);
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
            assert!(bad.is_null());
            let mut list: *mut c_char = NonNull::dangling().as_ptr();
            let r = platform_wallet_mnemonic_word_list(10, &mut list);
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
            assert!(list.is_null());
        }

        // validate: any supported language, but not a bad checksum or gibberish
        for (phrase, valid) in [
            (TEST_PLATFORM_WALLET_MNEMONIC, true),
            (TEST_FRENCH_MNEMONIC, true),
            ("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon", false),
            (TEST_INVALID_MNEMONIC, false),
        ] {
            let phrase = CString::new(phrase).unwrap();
            let r = unsafe { platform_wallet_mnemonic_validate(phrase.as_ptr()) };
            let expected = if valid {
                PlatformWalletFFIResultCode::Success
            } else {
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            };
            assert_eq!(r.code, expected, "{phrase:?}");
        }
        let r = unsafe { platform_wallet_mnemonic_validate(ptr::null()) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        // generate NULL out-pointer -> error, free NULL -> no-op
        unsafe {
            let r = platform_wallet_mnemonic_generate(
                12,
                FFIMnemonicLanguage::English as u32,
                ptr::null_mut(),
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            platform_wallet_mnemonic_free(ptr::null_mut());
        }
    }
}
