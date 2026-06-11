//! BIP-39 word/phrase helpers for the dashwallet Restore-Wallet flow.
//!
//! Faithful port of DashSync's `DSBIP39Mnemonic` helpers so dashwallet-ios
//! can drop `DSBIP39Mnemonic` from the recover model (migration row #4
//! sister methods). Parity reference (read alongside this file):
//! `DashSync/shared/Models/Wallet/DSBIP39Mnemonic.m`:
//!   - `wordIsValid:`      (m:258-261) -> [`platform_wallet_mnemonic_word_is_valid`]
//!   - `wordIsLocal:`      (m:278-281) -> generalized to
//!                                        [`platform_wallet_mnemonic_word_is_in_language`]
//!   - `normalizePhrase:`  (m:387-408) -> [`platform_wallet_mnemonic_normalize_phrase`]
//!   - `cleanupPhrase:`    (m:325-384) -> [`platform_wallet_mnemonic_cleanup_phrase`]
//!
//! This module is a thin **facade** over `key-wallet`'s BIP-39 primitives:
//! word membership, phrase normalization, and per-language validation all
//! delegate to `key_wallet::Mnemonic` (one source of BIP-39 logic — no direct
//! `bip39` dependency here). Recover-flow-specific policy stays local: the
//! DashSync bundled-language set ([`LANGS`]) and `cleanupPhrase`'s CJK
//! ideographic auto-split.
//!
//! Parity notes:
//! * DashSync compares against NFKD/lowercase wordlists with exact
//!   `containsObject:` membership and normalizes the *phrase* (not the
//!   per-word lookup). We mirror that: word lookups go through
//!   `key_wallet::Mnemonic::is_word_in_language` (exact membership against
//!   the canonical BIP-39 lists); callers normalize first via
//!   `key_wallet::Mnemonic::normalize_phrase`.
//! * DashSync bundles 7 languages (en, fr, es, it, ja, ko, zh-Hans) and
//!   `wordIsValid:` checks the union; we restrict [`LANGS`] to those 7 for
//!   exact parity rather than all 10 key-wallet languages.
//! * `phrase_is_valid_impl` rejects sub-12-word phrases (3/6/9 words) that
//!   DashSync's `phraseIsValid:` checksum loop would accept: key-wallet's
//!   `validate` (via bip39 `parse_in`) enforces the BIP-39 ≥12-word
//!   (≥128-bit entropy) floor. The divergence is intentional and inert — the
//!   only caller is `cleanup_phrase_impl`'s early-return gate, output is
//!   byte-identical for ASCII input (the CJK loop is a no-op without ≥U+3000
//!   chars), and the production recover path enforces
//!   `DW_PHRASE_MIN_LENGTH = 12` anyway.

use key_wallet::Language as L;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

use crate::error::PlatformWalletFFIResult;
use crate::{check_ptr, unwrap_result_or_return};

/// Ideographic space (U+3000), inserted between CJK words by `cleanupPhrase`.
/// DashSync uses the same `IDEO_SP` (`"\xE3\x80\x80"`).
const IDEO_SP: &str = "\u{3000}";

/// The 7 languages DashSync bundles as `BIP39Words.plist` localizations.
/// `wordIsValid` checks the union. (DashSync's English-only `wordIsLocal` is
/// generalized to `word_is_in_language`, so the caller picks the language.)
const LANGS: [L; 7] = [
    L::English,
    L::French,
    L::Spanish,
    L::Italian,
    L::Japanese,
    L::Korean,
    L::ChineseSimplified,
];

// ---------------------------------------------------------------------------
// Internal algorithm core (no FFI) — ports DSBIP39Mnemonic.m exactly.
// ---------------------------------------------------------------------------

/// `wordIsValid:` (m:258-261) — member of *any* bundled language. Exact
/// membership via key-wallet (canonical BIP-39 lists); caller pre-normalizes.
fn word_in_any_list(w: &str) -> bool {
    LANGS
        .iter()
        .any(|&l| key_wallet::Mnemonic::is_word_in_language(w, l))
}

/// Map a BCP-47-ish language code to a key-wallet `Language`. Covers all 10
/// key-wallet wordlists; `None` for an unrecognized code (the FFI entry point
/// then reports the word as not-in-language).
fn language_from_code(code: &str) -> Option<L> {
    match code.to_ascii_lowercase().as_str() {
        "en" | "english" => Some(L::English),
        "fr" | "french" => Some(L::French),
        "es" | "spanish" => Some(L::Spanish),
        "it" | "italian" => Some(L::Italian),
        "ja" | "japanese" => Some(L::Japanese),
        "ko" | "korean" => Some(L::Korean),
        "pt" | "portuguese" => Some(L::Portuguese),
        "cs" | "czech" => Some(L::Czech),
        "zh-hans" | "zh" | "zh-cn" | "chinesesimplified" => Some(L::ChineseSimplified),
        "zh-hant" | "zh-tw" | "chinesetraditional" => Some(L::ChineseTraditional),
        _ => None,
    }
}

/// `phraseIsValid:` (m:284-298) — true if the (already-normalized) phrase
/// decodes (all words present + valid checksum) in *some* bundled language.
/// Per-language loop, never autodetect. `key_wallet::Mnemonic::validate`
/// re-runs NFKD internally (via bip39 `parse_in`); that is idempotent on an
/// already-normalized phrase, so the pre-normalize step above it must stay.
fn phrase_is_valid_impl(normalized: &str) -> bool {
    LANGS
        .iter()
        .any(|&l| key_wallet::Mnemonic::validate(normalized, l))
}

/// `cleanupPhrase:` (m:325-384) — minimal cleanup for display/editing, plus
/// CJK ideographic auto-splitting. Returns the pre-normalize cleaned string.
///
/// Index note: DashSync indexes with UTF-16 units (`characterAtIndex:`,
/// `substringWithRange:`); we use Unicode scalars (`char`). Every BIP-39 CJK
/// word is in the BMP (1 char = 1 UTF-16 unit), so the two indexings agree.
fn cleanup_phrase_impl(phrase: &str) -> String {
    // (1) remove chars not in (letter ∪ mark ∪ whitespace). DashSync uses
    //     `letterCharacterSet` (Unicode L* AND M*) ∪ whitespaceAndNewline,
    //     inverted. We mirror M* via `is_combining_mark` so NFKD-decomposed
    //     input keeps its combining marks (e.g. Japanese voiced kana か+゙,
    //     Latin diacritics) instead of being corrupted.
    let mut s: String = phrase
        .chars()
        .filter(|&c| c.is_alphabetic() || is_combining_mark(c) || c.is_whitespace())
        .collect();

    // (2) newlines -> spaces
    s = s.replace('\n', " ");

    // (3) collapse "  " -> " "
    while s.contains("  ") {
        s = s.replace("  ", " ");
    }

    // (4) trim leading whitespace only (DashSync deletes index-0 ws in a loop)
    let mut s = s.trim_start().to_string();

    // (5) normalize + validity check; if valid, return the cleaned (pre-
    //     normalize) string verbatim — DashSync `return s;`
    let normalized = key_wallet::Mnemonic::normalize_phrase(&s);
    if phrase_is_valid_impl(&normalized) {
        return s;
    }

    // (6) CJK auto-split: walk the words of the *normalized* phrase; for each
    //     word starting at/after U+3000 that isn't already a whole valid word,
    //     scan substrings (len ≤ 8) and wrap valid matches in `s` with U+3000.
    //
    //     `s` still holds the caller's original Unicode form (e.g. NFC, which
    //     iOS Japanese IMEs emit), but the candidates below are sliced from
    //     `normalized` (NFKD). `String::replace` is exact-byte, so an NFKD
    //     candidate never matches an NFC `s` and nothing would be wrapped — the
    //     no-space phrase would come back unsplit. NFKD `s` here so the replace
    //     can hit. NFKD is the BIP-39 canonical form (the wordlist + `to_seed`
    //     use it), so emitting it from the split path is correct. (DSBIP39-
    //     Mnemonic has the same original-vs-NFKD mismatch; this fixes it.)
    s = s.nfkd().collect::<String>();

    let dbl_ideo = format!("{IDEO_SP}{IDEO_SP}");
    for word in normalized.split(' ') {
        let wchars: Vec<char> = word.chars().collect();
        if wchars.is_empty() {
            continue;
        }
        if (wchars[0] as u32) < 0x3000 || word_in_any_list(word) {
            continue;
        }

        let wlen = wchars.len();
        let mut i = 0usize;
        while i < wlen {
            let mut j = core::cmp::min(8, wlen - i);
            while j >= 1 {
                let candidate: String = wchars[i..i + j].iter().collect();
                if word_in_any_list(&candidate) {
                    let wrapped = format!("{IDEO_SP}{candidate}{IDEO_SP}");
                    s = s.replace(&candidate, &wrapped);
                    while s.contains(&dbl_ideo) {
                        s = s.replace(&dbl_ideo, IDEO_SP);
                    }
                    // CFStringTrimWhitespace strips leading/trailing ws,
                    // incl. U+3000 (which `str::trim` also treats as ws).
                    s = s.trim().to_string();
                    i += j - 1; // outer `i += 1` advances past the match
                    break;
                }
                j -= 1;
            }
            i += 1;
        }
    }

    s
}

// ---------------------------------------------------------------------------
// FFI surface
// ---------------------------------------------------------------------------

/// `true` if `word` is a BIP-39 word in any bundled language (DashSync
/// `wordIsValid:`). NULL / invalid-UTF-8 input returns `false` (the recover
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
        Ok(w) => word_in_any_list(w),
        Err(_) => false,
    }
}

/// `true` if `word` is a BIP-39 word in the given `language` (exact wordlist
/// membership; caller pre-normalizes). `language` is a BCP-47-ish code such as
/// `"en"`, `"ja"`, or `"zh-hans"`. NULL / invalid-UTF-8 / unrecognized-language
/// input returns `false`.
///
/// Replaces the former `platform_wallet_mnemonic_word_is_local`: which language
/// is "local" is an app-level choice, so the caller passes it explicitly
/// (wraps key-wallet's `is_word_in_language`, per review feedback).
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
    match language_from_code(code) {
        Some(lang) => key_wallet::Mnemonic::is_word_in_language(w, lang),
        None => false,
    }
}

/// NFKD + lowercase + whitespace-collapse (DashSync `normalizePhrase:`).
/// On success the caller owns `*out_string` and must free it via
/// [`crate::xpub_render::platform_wallet_free_string`].
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

/// Minimal cleanup + CJK auto-split (DashSync `cleanupPhrase:`). On success the
/// caller owns `*out_string` and must free it via
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
    let cleaned = cleanup_phrase_impl(p);
    let c = unwrap_result_or_return!(CString::new(cleaned));
    *out_string = c.into_raw();
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PlatformWalletFFIResultCode;
    use crate::xpub_render::platform_wallet_free_string;
    use unicode_normalization::UnicodeNormalization;

    /// First word of a language's all-zero-entropy 12-word phrase — a real
    /// wordlist entry for that language (index-0 word for zero entropy).
    fn first_word(lang: L) -> String {
        key_wallet::Mnemonic::from_entropy(&[0u8; 16], lang)
            .unwrap()
            .phrase()
            .split(' ')
            .next()
            .unwrap()
            .to_string()
    }

    /// Test conveniences. The production facade now calls key-wallet directly
    /// (no pass-through wrappers); these keep the existing test bodies readable.
    fn normalize_phrase_impl(input: &str) -> String {
        key_wallet::Mnemonic::normalize_phrase(input)
    }
    fn word_in_english(w: &str) -> bool {
        key_wallet::Mnemonic::is_word_in_language(w, L::English)
    }

    const EN_ZERO: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn english_known_vector() {
        assert!(phrase_is_valid_impl(&normalize_phrase_impl(EN_ZERO)));
        assert!(word_in_any_list("abandon"));
        assert!(word_in_english("abandon"));
        assert!(!word_in_any_list("notaword"));
    }

    #[test]
    fn non_english_word_valid_but_not_local() {
        // A Japanese wordlist entry: valid in the union, not in English.
        let w = first_word(L::Japanese);
        assert!(word_in_any_list(&w));
        assert!(!word_in_english(&w));
    }

    #[test]
    fn word_in_language_is_language_specific() {
        // Membership is per-language, not a global union.
        assert!(key_wallet::Mnemonic::is_word_in_language(
            "abandon",
            L::English
        ));
        assert!(!key_wallet::Mnemonic::is_word_in_language(
            "abandon",
            L::Japanese
        ));
        let ja = first_word(L::Japanese);
        assert!(key_wallet::Mnemonic::is_word_in_language(&ja, L::Japanese));
        assert!(!key_wallet::Mnemonic::is_word_in_language(&ja, L::English));
        // Code mapping: recognized codes resolve (case-insensitive); else None.
        assert!(matches!(language_from_code("EN"), Some(L::English)));
        assert!(matches!(
            language_from_code("zh-Hans"),
            Some(L::ChineseSimplified)
        ));
        assert!(matches!(language_from_code("ja"), Some(L::Japanese)));
        assert!(language_from_code("xx").is_none());
        assert!(language_from_code("").is_none());
    }

    #[test]
    fn normalize_trims_lowercases_collapses() {
        assert_eq!(
            normalize_phrase_impl("  ABANDON\tabout \n legal  "),
            "abandon about legal"
        );
        assert_eq!(normalize_phrase_impl(""), "");
        assert_eq!(normalize_phrase_impl("   "), "");
    }

    #[test]
    fn cleanup_strips_punctuation_and_passes_valid_through() {
        let dirty = "abandon, abandon. abandon abandon abandon abandon abandon abandon abandon abandon abandon about!";
        let cleaned = cleanup_phrase_impl(dirty);
        assert!(!cleaned.contains(','));
        assert!(!cleaned.contains('.'));
        assert!(!cleaned.contains('!'));
        // valid branch returns a string that normalizes to the valid phrase
        assert!(phrase_is_valid_impl(&normalize_phrase_impl(&cleaned)));
    }

    #[test]
    fn cjk_passthrough_and_autosplit() {
        // Distinct-word valid Japanese phrase (varied entropy avoids the
        // repeated-word ambiguity that defeats *any* greedy re-splitter,
        // DashSync's included).
        let entropy = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a,
            0x69, 0x78,
        ];
        let spaced = key_wallet::Mnemonic::from_entropy(&entropy, L::Japanese)
            .unwrap()
            .phrase();
        assert!(
            phrase_is_valid_impl(&normalize_phrase_impl(&spaced)),
            "fixture must be a valid Japanese phrase"
        );

        // (a) a space-separated valid CJK phrase passes the valid branch through
        assert!(phrase_is_valid_impl(&normalize_phrase_impl(
            &cleanup_phrase_impl(&spaced)
        )));

        // (b) a no-space CJK phrase gets ideographic spaces inserted (best-effort
        //     split — exact reconstruction is greedy-dependent, matching DashSync).
        let nospace: String = spaced.split(' ').collect();
        assert!(
            cleanup_phrase_impl(&nospace).contains(IDEO_SP),
            "cleanup should insert ideographic spaces into a no-space CJK phrase"
        );
    }

    #[test]
    fn nfc_japanese_no_space_autosplit() {
        // Regression guard for the NFC/NFKD mismatch in `cleanup_phrase_impl`:
        // iOS Japanese IMEs emit precomposed (NFC) text, while the BIP-39
        // wordlist is NFKD. A no-space NFC Japanese phrase must still auto-split
        // — the CJK loop NFKDs the working buffer so the exact-byte replace can
        // hit. The other CJK fixtures build from `from_entropy(..).phrase()`,
        // which is already NFKD, so only this NFC fixture exercises the bug.
        let entropy = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a,
            0x69, 0x78,
        ];
        let phrase = key_wallet::Mnemonic::from_entropy(&entropy, L::Japanese)
            .unwrap()
            .phrase();
        // No-space, in both Unicode forms.
        let nfkd_nospace: String = phrase.split(' ').collect();
        let nfc_nospace: String = nfkd_nospace.nfc().collect();
        // The fixture must actually be NFC (different bytes) or it wouldn't
        // exercise the mismatch — this phrase contains voiced kana (dakuten).
        assert_ne!(
            nfc_nospace, nfkd_nospace,
            "fixture must be NFC (distinct from NFKD) to cover the bug"
        );

        let from_nfc = cleanup_phrase_impl(&nfc_nospace);
        let from_nfkd = cleanup_phrase_impl(&nfkd_nospace);
        assert!(
            from_nfc.contains(IDEO_SP),
            "NFC no-space Japanese phrase should still get ideographic spaces"
        );
        // The crux: NFC input must auto-split *identically* to the equivalent
        // NFKD input. `.contains(IDEO_SP)` alone is too weak — the unvoiced
        // words in the phrase split fine even without the fix; only the voiced
        // (NFC-precomposed) words are skipped. Comparing the two full splits
        // catches that. Once the fix NFKDs the working buffer the NFC path
        // becomes byte-identical to the NFKD path; without it the voiced words
        // stay glued and the splits diverge.
        assert_eq!(
            from_nfc, from_nfkd,
            "NFC input must auto-split the same as NFKD input"
        );
    }

    #[test]
    fn invalid_checksum_phrase_rejected() {
        // valid words, wrong checksum
        let bad = "bless cloud wheel regular tiny venue bird web grief security dignity zoo";
        assert!(!phrase_is_valid_impl(&normalize_phrase_impl(bad)));
    }

    #[test]
    fn empty_word_is_invalid() {
        assert!(!word_in_any_list(""));
        assert!(!word_in_english(""));
    }

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

    // ----- Exotic-language coverage (DashSync's tests only cover English +
    // one Czech normalization case; these add the per-language validity and
    // CJK coverage DashSync lacks). -----

    #[test]
    fn every_bundled_language_word_validates_in_union() {
        // A representative word from each bundled language is valid in the
        // union (`wordIsValid`); only English words are "local" (`wordIsLocal`).
        for lang in LANGS {
            let w = first_word(lang);
            assert!(word_in_any_list(&w), "{lang:?} word should be valid");
        }
        assert!(word_in_english(&first_word(L::English)));
        assert!(!word_in_english(&first_word(L::Japanese)));
        assert!(!word_in_english(&first_word(L::ChineseSimplified)));
    }

    #[test]
    fn valid_phrase_per_bundled_language() {
        // A real 12-word phrase in each non-English bundled language validates
        // through the per-language decode loop (`phraseIsValid`).
        let entropy = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a,
            0x69, 0x78,
        ];
        for lang in [
            L::French,
            L::Spanish,
            L::Italian,
            L::Japanese,
            L::Korean,
            L::ChineseSimplified,
        ] {
            let phrase = key_wallet::Mnemonic::from_entropy(&entropy, lang)
                .unwrap()
                .phrase();
            assert!(
                phrase_is_valid_impl(&normalize_phrase_impl(&phrase)),
                "{lang:?} phrase should validate"
            );
        }
    }

    #[test]
    fn normalize_collapses_unicode_forms_like_dashsync() {
        // Mirrors DSBIP39Tests.m:290-314 (a Czech phrase in NFKD/NFC/NFD all
        // derive the same seed) at the normalize layer: every Unicode form
        // normalizes to the identical string.
        let nfkd = "Pr\u{30c}i\u{301}s\u{30c}erne\u{30c} z\u{30c}lut\u{30c}ouc\u{30c}ky\u{301} ku\u{30a}n\u{30c} u\u{301}pe\u{30c}l d\u{30c}a\u{301}belske\u{301} o\u{301}dy za\u{301}ker\u{30c}ny\u{301} uc\u{30c}en\u{30c} be\u{30c}z\u{30c}i\u{301} pode\u{301}l zo\u{301}ny u\u{301}lu\u{30a}";
        let nfc = "P\u{159}\u{ed}\u{161}ern\u{11b} \u{17e}lu\u{165}ou\u{10d}k\u{fd} k\u{16f}\u{148} \u{fa}p\u{11b}l \u{10f}\u{e1}belsk\u{e9} \u{f3}dy z\u{e1}ke\u{159}n\u{fd} u\u{10d}e\u{148} b\u{11b}\u{17e}\u{ed} pod\u{e9}l z\u{f3}ny \u{fa}l\u{16f}";
        let nfd: String = nfc.nfd().collect();
        assert_eq!(normalize_phrase_impl(nfkd), normalize_phrase_impl(nfc));
        assert_eq!(normalize_phrase_impl(nfkd), normalize_phrase_impl(&nfd));
    }

    #[test]
    fn chinese_no_space_autosplit() {
        // Simplified-Chinese words are single ideographs; a no-space phrase
        // must get ideographic spaces inserted by cleanup.
        let entropy = [
            0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x07, 0x18, 0x29, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e,
            0x8f, 0x90,
        ];
        let nospace: String = key_wallet::Mnemonic::from_entropy(&entropy, L::ChineseSimplified)
            .unwrap()
            .phrase()
            .split(' ')
            .collect();
        assert!(
            cleanup_phrase_impl(&nospace).contains(IDEO_SP),
            "no-space Chinese phrase should get ideographic spaces"
        );
    }

    #[test]
    fn english_trezor_vectors_validate() {
        // Canonical English BIP-39 vectors from DashSync's own test suite
        // (DSBIP39Tests.m), covering 12- and 24-word lengths: each validates,
        // and every word is both globally valid and English-local.
        let phrases = [
            "legal winner thank year wave sausage worth useful legal winner thank yellow",
            "letter advice cage absurd amount doctor acoustic avoid letter advice cage above",
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
            "jelly better achieve collect unaware mountain thought cargo oxygen act hood bridge",
            "exile ask congress lamp submit jacket era scheme attend cousin alcohol catch course end lucky hurt sentence oven short ball bird grab wing top",
        ];
        for p in phrases {
            let norm = normalize_phrase_impl(p);
            assert!(phrase_is_valid_impl(&norm), "should be valid: {p}");
            for word in norm.split(' ') {
                assert!(word_in_any_list(word), "word should be valid: {word}");
                assert!(
                    word_in_english(word),
                    "word should be English-local: {word}"
                );
            }
        }
    }

    #[test]
    fn english_messy_input_normalizes_and_validates() {
        // Mixed case + tabs/newlines/extra spaces collapse to a canonical phrase.
        let messy = "  ABANDON\tAbandon abandon  abandon abandon abandon abandon abandon abandon abandon abandon ABOUT \n";
        assert_eq!(
            normalize_phrase_impl(messy),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        );
        assert!(phrase_is_valid_impl(&normalize_phrase_impl(messy)));
    }

    #[test]
    fn word_validity_requires_prenormalized_input() {
        // Wordlist membership is exact-byte; callers MUST normalize first
        // (DashSync normalizes the phrase before every containsObject: check).
        // Raw uppercase fails; the normalized form passes. The NFC/NFD
        // accent-convergence half of this contract is covered in key-wallet's
        // `test_normalize_phrase_unicode_forms_converge` / `test_is_word_in_language`,
        // where the wordlist is accessible.
        assert!(!word_in_english("ABANDON"));
        assert!(word_in_english(&normalize_phrase_impl("ABANDON")));
        assert!(!word_in_any_list("ABANDON"));
        assert!(word_in_any_list(&normalize_phrase_impl("ABANDON")));
    }

    #[test]
    fn cleanup_strips_punctuation_around_cjk() {
        // Punctuation removal (step 1) and CJK auto-split must both fire on a
        // no-space CJK phrase that arrives with ASCII punctuation interleaved.
        let entropy = [
            0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x07, 0x18, 0x29, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e,
            0x8f, 0x90,
        ];
        let phrase = key_wallet::Mnemonic::from_entropy(&entropy, L::ChineseSimplified)
            .unwrap()
            .phrase();
        let dirty = phrase.split(' ').collect::<Vec<_>>().join(",");
        let cleaned = cleanup_phrase_impl(&dirty);
        assert!(
            !cleaned.contains(','),
            "punctuation must be stripped: {cleaned:?}"
        );
        assert!(
            cleaned.contains(IDEO_SP),
            "CJK split should insert ideographic spaces: {cleaned:?}"
        );
    }

    #[test]
    fn normalize_is_idempotent() {
        for input in [
            "  ABANDON\tabout \n legal  ",
            "abandon about legal",
            "",
            "   ",
            "Pr\u{30c}i\u{301}s\u{30c}erne\u{30c}",
        ] {
            let once = normalize_phrase_impl(input);
            assert_eq!(
                normalize_phrase_impl(&once),
                once,
                "normalize must be idempotent for {input:?}"
            );
        }
    }

    #[test]
    fn cleanup_wraps_all_occurrences_of_repeated_cjk_word() {
        // The CJK split uses a global String::replace (parity with DashSync's
        // stringByReplacingOccurrencesOfString:), so every occurrence of a
        // repeated word is wrapped; the loop must terminate without panic.
        let cw = first_word(L::ChineseSimplified); // a single valid ideograph
        let nospace = format!("{cw}{cw}{cw}");
        let cleaned = cleanup_phrase_impl(&nospace);
        assert!(
            cleaned.contains(IDEO_SP),
            "repeated CJK word should get ideographic spaces: {cleaned:?}"
        );
        assert_eq!(
            cleaned.matches(cw.as_str()).count(),
            3,
            "all occurrences preserved (wrap-all, no loss/dup): {cleaned:?}"
        );
    }
}
