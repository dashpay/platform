//! BIP-39 word/phrase helpers for the dashwallet Restore-Wallet flow.
//!
//! Faithful port of DashSync's `DSBIP39Mnemonic` helpers so dashwallet-ios
//! can drop `DSBIP39Mnemonic` from the recover model (migration row #4
//! sister methods). Parity reference (read alongside this file):
//! `DashSync/shared/Models/Wallet/DSBIP39Mnemonic.m`:
//!   - `wordIsValid:`      (m:258-261) -> [`platform_wallet_mnemonic_word_is_valid`]
//!   - `wordIsLocal:`      (m:278-281) -> [`platform_wallet_mnemonic_word_is_local`]
//!   - `normalizePhrase:`  (m:387-408) -> [`platform_wallet_mnemonic_normalize_phrase`]
//!   - `cleanupPhrase:`    (m:325-384) -> [`platform_wallet_mnemonic_cleanup_phrase`]
//!
//! Parity notes:
//! * DashSync compares against NFKD/lowercase wordlists with exact
//!   `containsObject:` membership and normalizes the *phrase* (not the
//!   per-word lookup). We mirror that: word lookups are exact-byte against
//!   the bip39 wordlists (same canonical NFKD/lowercase lists); callers
//!   normalize first via `normalizePhrase`/`cleanupPhrase`.
//! * DashSync bundles 7 languages (en, fr, es, it, ja, ko, zh-Hans) and
//!   `wordIsValid:` checks the union; we restrict [`LANGS`] to those 7 for
//!   exact parity rather than all 10 bip39 languages.
//! * `phrase_is_valid_impl` rejects sub-12-word phrases (3/6/9 words) that
//!   DashSync's `phraseIsValid:` checksum loop would accept: bip39's
//!   `parse_in_normalized` enforces the BIP-39 ≥12-word (≥128-bit entropy)
//!   floor. The divergence is intentional and inert — the only caller is
//!   `cleanup_phrase_impl`'s early-return gate, output is byte-identical for
//!   ASCII input (the CJK loop is a no-op without ≥U+3000 chars), and the
//!   production recover path enforces `DW_PHRASE_MIN_LENGTH = 12` anyway.

use bip39::Language as B;
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
/// `wordIsValid` checks the union; `wordIsLocal` uses only English (the
/// recover flow's default language).
const LANGS: [B; 7] = [
    B::English,
    B::French,
    B::Spanish,
    B::Italian,
    B::Japanese,
    B::Korean,
    B::SimplifiedChinese,
];

// ---------------------------------------------------------------------------
// Internal algorithm core (no FFI) — ports DSBIP39Mnemonic.m exactly.
// ---------------------------------------------------------------------------

/// Exact membership of `w` in one language's wordlist (DashSync plist
/// `containsObject:`). The bip39 wordlists are the canonical NFKD/lowercase
/// BIP-39 lists, identical to DashSync's plists.
fn word_in_list(lang: B, w: &str) -> bool {
    lang.word_list().contains(&w)
}

/// `wordIsValid:` (m:258-261) — member of *any* bundled language.
fn word_in_any_list(w: &str) -> bool {
    LANGS.iter().any(|&l| word_in_list(l, w))
}

/// `wordIsLocal:` (m:278-281) — member of the default (English) wordlist.
fn word_in_english(w: &str) -> bool {
    word_in_list(B::English, w)
}

/// NFKD + lowercase, matching DashSync's `CFStringNormalize(kCFStringNormalizationFormKD)`
/// + `CFStringLowercase(CFLocaleGetSystem())`.
fn nfkd_lower(s: &str) -> String {
    s.nfkd().collect::<String>().to_lowercase()
}

/// `normalizePhrase:` (m:387-408) — NFKD, lowercase, trim, collapse every
/// whitespace run to a single space. `split_whitespace().join(" ")` performs
/// the trim + "replace each whitespace char with a space" + "collapse double
/// spaces" in one pass (Unicode `White_Space` ≈ NSCharacterSet
/// whitespaceAndNewline for typed input, incl. U+3000).
fn normalize_phrase_impl(input: &str) -> String {
    nfkd_lower(input)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// `phraseIsValid:` (m:284-298) — true if the (already-normalized) phrase
/// decodes (all words present + valid checksum) in *some* bundled language.
/// Per-language loop, never bip39 autodetect (`parse()`/`language_of` returns
/// `AmbiguousLanguages` and diverges from DashSync's per-language decode).
fn phrase_is_valid_impl(normalized: &str) -> bool {
    LANGS
        .iter()
        .any(|&l| bip39::Mnemonic::parse_in_normalized(l, normalized).is_ok())
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
    let normalized = normalize_phrase_impl(&s);
    if phrase_is_valid_impl(&normalized) {
        return s;
    }

    // (6) CJK auto-split: walk the words of the *normalized* phrase; for each
    //     word starting at/after U+3000 that isn't already a whole valid word,
    //     scan substrings (len ≤ 8) and wrap valid matches in `s` with U+3000.
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

/// `true` if `word` is in the default (English) wordlist (DashSync
/// `wordIsLocal:`). NULL / invalid-UTF-8 input returns `false`.
///
/// # Safety
/// `word` must be NULL or a valid null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_mnemonic_word_is_local(word: *const c_char) -> bool {
    if word.is_null() {
        return false;
    }
    match CStr::from_ptr(word).to_str() {
        Ok(w) => word_in_english(w),
        Err(_) => false,
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
    let normalized = normalize_phrase_impl(p);
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
        let w = B::Japanese.word_list()[0];
        assert!(word_in_any_list(w));
        assert!(!word_in_english(w));
    }

    #[test]
    fn normalize_trims_lowercases_collapses() {
        assert_eq!(normalize_phrase_impl("  ABANDON\tabout \n legal  "), "abandon about legal");
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
        let m = bip39::Mnemonic::from_entropy_in(B::Japanese, &entropy).unwrap();
        let words: Vec<&'static str> = m.words().collect();
        let spaced = words.join(" ");
        assert!(
            phrase_is_valid_impl(&normalize_phrase_impl(&spaced)),
            "fixture must be a valid Japanese phrase"
        );

        // (a) a space-separated valid CJK phrase passes the valid branch through
        assert!(phrase_is_valid_impl(&normalize_phrase_impl(&cleanup_phrase_impl(&spaced))));

        // (b) a no-space CJK phrase gets ideographic spaces inserted (best-effort
        //     split — exact reconstruction is greedy-dependent, matching DashSync).
        let nospace: String = words.concat();
        assert!(
            cleanup_phrase_impl(&nospace).contains(IDEO_SP),
            "cleanup should insert ideographic spaces into a no-space CJK phrase"
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
        unsafe {
            assert!(platform_wallet_mnemonic_word_is_valid(valid.as_ptr()));
            assert!(!platform_wallet_mnemonic_word_is_valid(invalid.as_ptr()));
            assert!(platform_wallet_mnemonic_word_is_local(valid.as_ptr()));
            assert!(!platform_wallet_mnemonic_word_is_valid(std::ptr::null()));
            assert!(!platform_wallet_mnemonic_word_is_local(std::ptr::null()));
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
            let w = lang.word_list()[0];
            assert!(word_in_any_list(w), "{lang:?} word_list[0] should be valid");
        }
        assert!(word_in_english(B::English.word_list()[0]));
        assert!(!word_in_english(B::Japanese.word_list()[0]));
        assert!(!word_in_english(B::SimplifiedChinese.word_list()[0]));
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
            B::French,
            B::Spanish,
            B::Italian,
            B::Japanese,
            B::Korean,
            B::SimplifiedChinese,
        ] {
            let m = bip39::Mnemonic::from_entropy_in(lang, &entropy).unwrap();
            let phrase = m.words().collect::<Vec<_>>().join(" ");
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
        let m = bip39::Mnemonic::from_entropy_in(B::SimplifiedChinese, &entropy).unwrap();
        let nospace: String = m.words().collect::<Vec<_>>().concat();
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
                assert!(word_in_english(word), "word should be English-local: {word}");
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
        // Wordlist membership is exact-byte; callers MUST nfkd_lower first
        // (DashSync normalizes the phrase before every containsObject: check).

        // (a) Uppercase: the raw form fails, the normalized form passes.
        assert!(!word_in_english("ABANDON"));
        assert!(word_in_english(&nfkd_lower("ABANDON")));
        assert!(!word_in_any_list("ABANDON"));
        assert!(word_in_any_list(&nfkd_lower("ABANDON")));

        // (b) Accent: find a bundled word whose NFC and NFD forms differ
        //     (Japanese voiced kana / Latin diacritics guarantee a hit). Both
        //     forms converge to the same valid member under nfkd_lower. We
        //     assert only the positive normalize->member direction, so the test
        //     is independent of which Unicode form the crate stores its lists in.
        let mut accented: Option<&'static str> = None;
        'outer: for lang in LANGS {
            for &word in lang.word_list().iter() {
                let nfc: String = word.nfc().collect();
                let nfd: String = word.nfd().collect();
                if nfc != nfd {
                    accented = Some(word);
                    break 'outer;
                }
            }
        }
        let w = accented.expect("some bundled word should have distinct NFC/NFD forms");
        let nfc: String = w.nfc().collect();
        let nfd: String = w.nfd().collect();
        assert_ne!(nfc, nfd);
        assert_eq!(nfkd_lower(&nfc), nfkd_lower(&nfd));
        assert!(word_in_any_list(&nfkd_lower(&nfc)));
        assert!(word_in_any_list(&nfkd_lower(&nfd)));
    }

    #[test]
    fn cleanup_strips_punctuation_around_cjk() {
        // Punctuation removal (step 1) and CJK auto-split must both fire on a
        // no-space CJK phrase that arrives with ASCII punctuation interleaved.
        let entropy = [
            0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x07, 0x18, 0x29, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e,
            0x8f, 0x90,
        ];
        let m = bip39::Mnemonic::from_entropy_in(B::SimplifiedChinese, &entropy).unwrap();
        let words: Vec<&'static str> = m.words().collect();
        let dirty = words.join(",");
        let cleaned = cleanup_phrase_impl(&dirty);
        assert!(!cleaned.contains(','), "punctuation must be stripped: {cleaned:?}");
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
        let cw = B::SimplifiedChinese.word_list()[0]; // a single valid ideograph
        let nospace = format!("{cw}{cw}{cw}");
        let cleaned = cleanup_phrase_impl(&nospace);
        assert!(
            cleaned.contains(IDEO_SP),
            "repeated CJK word should get ideographic spaces: {cleaned:?}"
        );
        assert_eq!(
            cleaned.matches(cw).count(),
            3,
            "all occurrences preserved (wrap-all, no loss/dup): {cleaned:?}"
        );
    }
}
