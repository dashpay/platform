//! BIP-39 recover-flow phrase/word helpers — the recover-flow **policy** layer
//! over `key-wallet`'s BIP-39 primitives.
//!
//! Faithful port of DashSync's `DSBIP39Mnemonic` recover helpers (so
//! dashwallet-ios can drop `DSBIP39Mnemonic`). Word membership, normalization
//! and per-language validity all delegate to `key_wallet::Mnemonic`; what lives
//! here is the recover-flow-specific policy `key_wallet` does not encode: the
//! DashSync bundled-language set (`LANGS`) and `cleanup_phrase`'s CJK
//! ideographic auto-split. This module previously lived in
//! `rs-platform-wallet-ffi`; it moved here so the FFI crate is a thin C surface.
//!
//! Parity notes:
//! * DashSync compares against NFKD/lowercase wordlists with exact
//!   `containsObject:` membership and normalizes the *phrase* (not the per-word
//!   lookup). We mirror that: lookups go through
//!   `key_wallet::Mnemonic::is_word_in_language` (exact membership); callers
//!   normalize first via `key_wallet::Mnemonic::normalize_phrase`.
//! * DashSync bundles 7 languages (en, fr, es, it, ja, ko, zh-Hans); `LANGS`
//!   is those 7 for exact parity rather than all 10 key-wallet languages.
//! * `phrase_is_valid` rejects sub-12-word phrases that DashSync's checksum
//!   loop would accept (key-wallet's `validate` enforces the BIP-39 ≥12-word
//!   floor). Inert: its only caller is `cleanup_phrase`'s early-return gate.

use key_wallet::Language as L;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Ideographic space (U+3000), inserted between CJK words by `cleanup_phrase`
/// (DashSync's `IDEO_SP`).
const IDEO_SP: &str = "\u{3000}";

/// The 7 languages DashSync bundles as `BIP39Words.plist` localizations;
/// `word_in_any_list` checks their union.
const LANGS: [L; 7] = [
    L::English,
    L::French,
    L::Spanish,
    L::Italian,
    L::Japanese,
    L::Korean,
    L::ChineseSimplified,
];

/// `true` if `word` is a member of *any* bundled-language wordlist (DashSync
/// `wordIsValid:`). Exact membership via key-wallet; caller pre-normalizes.
pub fn word_in_any_list(word: &str) -> bool {
    LANGS
        .iter()
        .any(|&l| key_wallet::Mnemonic::is_word_in_language(word, l))
}

/// `true` if `word` is a member of `language_code`'s wordlist (exact
/// membership; caller pre-normalizes). `language_code` is a BCP-47-ish code
/// such as `"en"`, `"ja"`, or `"zh-hans"`; an unrecognized code returns
/// `false`. Which language is "local" is the app's choice — hence a parameter.
pub fn word_in_language(word: &str, language_code: &str) -> bool {
    match language_from_code(language_code) {
        Some(lang) => key_wallet::Mnemonic::is_word_in_language(word, lang),
        None => false,
    }
}

/// Map a BCP-47-ish language code to a key-wallet `Language`. Covers all 10
/// key-wallet wordlists; `None` for an unrecognized code.
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

/// `phraseIsValid:` — true if the (already-normalized) phrase decodes (all
/// words present + valid checksum) in *some* bundled language. Per-language
/// loop, never autodetect. `key_wallet::Mnemonic::validate` re-runs NFKD
/// internally (idempotent on an already-normalized phrase).
fn phrase_is_valid(normalized: &str) -> bool {
    LANGS
        .iter()
        .any(|&l| key_wallet::Mnemonic::validate(normalized, l))
}

/// `cleanupPhrase:` — minimal cleanup for display/editing plus CJK ideographic
/// auto-splitting. Returns the pre-normalize cleaned string.
///
/// Index note: DashSync indexes UTF-16 units (`characterAtIndex:`); we use
/// Unicode scalars (`char`). Every BIP-39 CJK word is in the BMP (1 char =
/// 1 UTF-16 unit), so the two indexings agree.
pub fn cleanup_phrase(phrase: &str) -> String {
    // (0) Bound pathological input: a real BIP-39 phrase is <= 24 words, well
    //     under 1 KiB. Past a generous byte cap, skip the superlinear CJK scan
    //     and return the input verbatim — a long no-space CJK paste would
    //     otherwise drive large allocations on the recover-UI thread. The UI
    //     enforces the real phrase length.
    const MAX_CLEANUP_BYTES: usize = 4096;
    if phrase.len() > MAX_CLEANUP_BYTES {
        return phrase.to_string();
    }

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
    if phrase_is_valid(&normalized) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_normalization::UnicodeNormalization;

    /// First word of a language's all-zero-entropy 12-word phrase — a real
    /// wordlist entry for that language.
    fn first_word(lang: L) -> String {
        key_wallet::Mnemonic::from_entropy(&[0u8; 16], lang)
            .unwrap()
            .phrase()
            .split(' ')
            .next()
            .unwrap()
            .to_string()
    }

    /// Test conveniences (the public fns call key-wallet directly).
    fn normalize_phrase_impl(input: &str) -> String {
        key_wallet::Mnemonic::normalize_phrase(input)
    }
    fn word_in_english(w: &str) -> bool {
        key_wallet::Mnemonic::is_word_in_language(w, L::English)
    }

    #[test]
    fn word_in_language_and_code_mapping() {
        // word_in_language resolves the BCP-47 code, then checks membership.
        assert!(word_in_language("abandon", "en"));
        assert!(!word_in_language("abandon", "ja"));
        let ja = first_word(L::Japanese);
        assert!(word_in_language(&ja, "ja"));
        assert!(!word_in_language(&ja, "en"));
        assert!(!word_in_language("abandon", "xx"));
        // language_from_code mapping (case-insensitive); unknown/empty -> None.
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
    fn cleanup_strips_punctuation_and_passes_valid_through() {
        let dirty = "abandon, abandon. abandon abandon abandon abandon abandon abandon abandon abandon abandon about!";
        let cleaned = cleanup_phrase(dirty);
        assert!(!cleaned.contains(','));
        assert!(!cleaned.contains('.'));
        assert!(!cleaned.contains('!'));
        // valid branch returns a string that normalizes to the valid phrase
        assert!(phrase_is_valid(&normalize_phrase_impl(&cleaned)));
    }

    #[test]
    fn cjk_passthrough_and_autosplit() {
        // Distinct-word valid Japanese phrase (varied entropy avoids the
        // repeated-word ambiguity that defeats any greedy re-splitter).
        let entropy = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a,
            0x69, 0x78,
        ];
        let spaced = key_wallet::Mnemonic::from_entropy(&entropy, L::Japanese)
            .unwrap()
            .phrase();
        assert!(
            phrase_is_valid(&normalize_phrase_impl(&spaced)),
            "fixture must be a valid Japanese phrase"
        );

        // (a) a space-separated valid CJK phrase passes the valid branch through
        assert!(phrase_is_valid(&normalize_phrase_impl(&cleanup_phrase(
            &spaced
        ))));

        // (b) a no-space CJK phrase gets ideographic spaces inserted
        let nospace: String = spaced.split(' ').collect();
        assert!(
            cleanup_phrase(&nospace).contains(IDEO_SP),
            "cleanup should insert ideographic spaces into a no-space CJK phrase"
        );
    }

    #[test]
    fn nfc_japanese_no_space_autosplit() {
        // Regression guard for the NFC/NFKD mismatch in cleanup_phrase: iOS
        // Japanese IMEs emit precomposed (NFC) text, the BIP-39 wordlist is
        // NFKD. A no-space NFC Japanese phrase must still auto-split — the CJK
        // loop NFKDs the working buffer so the exact-byte replace can hit.
        let entropy = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a,
            0x69, 0x78,
        ];
        let phrase = key_wallet::Mnemonic::from_entropy(&entropy, L::Japanese)
            .unwrap()
            .phrase();
        let nfkd_nospace: String = phrase.split(' ').collect();
        let nfc_nospace: String = nfkd_nospace.nfc().collect();
        // The fixture must actually be NFC (different bytes) to exercise the bug.
        assert_ne!(
            nfc_nospace, nfkd_nospace,
            "fixture must be NFC (distinct from NFKD) to cover the bug"
        );

        let from_nfc = cleanup_phrase(&nfc_nospace);
        let from_nfkd = cleanup_phrase(&nfkd_nospace);
        assert!(
            from_nfc.contains(IDEO_SP),
            "NFC no-space Japanese phrase should still get ideographic spaces"
        );
        // NFC input must auto-split *identically* to the equivalent NFKD input;
        // `.contains(IDEO_SP)` alone is too weak (unvoiced words split anyway).
        assert_eq!(
            from_nfc, from_nfkd,
            "NFC input must auto-split the same as NFKD input"
        );
    }

    #[test]
    fn every_bundled_language_word_validates_in_union() {
        // A representative word from each bundled language is valid in the
        // union; only English words are English-local.
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
        // through the per-language union (`phrase_is_valid`). (key-wallet also
        // covers per-language `validate` natively; this covers the union.)
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
                phrase_is_valid(&normalize_phrase_impl(&phrase)),
                "{lang:?} phrase should validate in the union"
            );
        }
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
            cleanup_phrase(&nospace).contains(IDEO_SP),
            "no-space Chinese phrase should get ideographic spaces"
        );
    }

    #[test]
    fn cleanup_strips_punctuation_around_cjk() {
        // Punctuation removal and CJK auto-split must both fire on a no-space
        // CJK phrase that arrives with ASCII punctuation interleaved.
        let entropy = [
            0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x07, 0x18, 0x29, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e,
            0x8f, 0x90,
        ];
        let phrase = key_wallet::Mnemonic::from_entropy(&entropy, L::ChineseSimplified)
            .unwrap()
            .phrase();
        let dirty = phrase.split(' ').collect::<Vec<_>>().join(",");
        let cleaned = cleanup_phrase(&dirty);
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
    fn cleanup_wraps_all_occurrences_of_repeated_cjk_word() {
        // The CJK split uses a global String::replace, so every occurrence of a
        // repeated word is wrapped; the loop must terminate without panic.
        let cw = first_word(L::ChineseSimplified); // a single valid ideograph
        let nospace = format!("{cw}{cw}{cw}");
        let cleaned = cleanup_phrase(&nospace);
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

    #[test]
    fn cleanup_phrase_caps_pathological_input() {
        // Past the byte cap, input is returned verbatim instead of driving the
        // superlinear CJK scan.
        let huge = "あ".repeat(5000); // ~15 KB, well over MAX_CLEANUP_BYTES
        assert_eq!(
            cleanup_phrase(&huge),
            huge,
            "input past the cap is returned unchanged"
        );
        // A normal-length phrase is still processed (not capped).
        let entropy = [
            0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x07, 0x18, 0x29, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e,
            0x8f, 0x90,
        ];
        let nospace: String = key_wallet::Mnemonic::from_entropy(&entropy, L::ChineseSimplified)
            .unwrap()
            .phrase()
            .split(' ')
            .collect();
        assert!(cleanup_phrase(&nospace).contains(IDEO_SP));
    }
}
