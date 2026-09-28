//! The `normalizedFrom` declaration of a string property: the property holds a
//! normalized form of another string property of the same document, such as a
//! name folded for case-insensitive, homograph-resistant uniqueness.
//!
//! The platform computes the property when a created or replaced document
//! leaves it out and supplies its source, and checks it when the document
//! supplies it. Which characters a value may hold is the job of the
//! properties' own `pattern`: a transform never refuses a character.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The `normalizedFrom` declaration of a string property.
///
/// Declared as
/// `"normalizedFrom": { "property": "<dotted property path>", "transform": "homographSafeASCII" }`
/// on a string property (meta-schema v3, protocol version 14). The declaring
/// property, the target, must hold `transform` applied to the value of
/// `property`, the source, and be absent exactly when the source is. The
/// parser checks at contract registration that the source is another string
/// property of the same document type, that neither side is transient, that
/// the source declares no `normalizedFrom` of its own, and that the source
/// sits inside every object that holds the target, so a document holding the
/// source always has somewhere to put the target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedFrom {
    /// The dotted path of the source string property.
    pub property: String,
    /// How the source is normalized.
    pub transform: NormalizationTransform,
}

impl NormalizedFrom {
    /// The value the declaring property must hold for a source value.
    pub fn normalize(&self, source: &str) -> String {
        self.transform.apply(source)
    }
}

/// How a [`NormalizedFrom`] property is computed from its source.
///
/// A closed list: every node must compute exactly the same string, so a
/// transform is defined character by character without any table that could
/// differ between builds (Unicode case mappings change between Rust
/// releases). Every transform keeps the source's byte length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalizationTransform {
    /// DPNS's label normalization over ASCII: `A` to `Z` become lowercase,
    /// then `o` becomes `0` and `i` and `l` become `1`. Every other
    /// character, ASCII or not, is kept as it is. On an ASCII value this is
    /// exactly `convert_to_homograph_safe_chars`; it resists homographs only
    /// when the properties' `pattern` restricts them to ASCII, as DPNS does.
    #[serde(rename = "homographSafeASCII")]
    HomographSafeAscii,
}

impl NormalizationTransform {
    /// Every transform, in wire-name order.
    pub const ALL: [NormalizationTransform; 1] = [NormalizationTransform::HomographSafeAscii];

    /// The wire name, the value of `normalizedFrom.transform`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            NormalizationTransform::HomographSafeAscii => "homographSafeASCII",
        }
    }

    /// The transform a wire name names, `None` for any other name.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|transform| transform.as_str() == name)
    }

    /// The normalized form of `source`.
    pub fn apply(&self, source: &str) -> String {
        match self {
            NormalizationTransform::HomographSafeAscii => source
                .chars()
                .map(|character| match character.to_ascii_lowercase() {
                    'o' => '0',
                    'i' | 'l' => '1',
                    other => other,
                })
                .collect(),
        }
    }
}

impl fmt::Display for NormalizationTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::strings::convert_to_homograph_safe_chars;

    const HOMOGRAPH_SAFE_ASCII: NormalizationTransform = NormalizationTransform::HomographSafeAscii;

    /// Every string of up to three characters over `alphabet`.
    fn strings_up_to_three(alphabet: &[char]) -> Vec<String> {
        let mut strings = vec![String::new()];
        let mut last: Vec<String> = vec![String::new()];
        for _ in 0..3 {
            last = last
                .iter()
                .flat_map(|prefix| {
                    alphabet.iter().map(move |character| {
                        let mut string = prefix.clone();
                        string.push(*character);
                        string
                    })
                })
                .collect();
            strings.extend(last.iter().cloned());
        }
        strings
    }

    #[test]
    fn should_match_the_dpns_trigger_normalization_on_every_ascii_character() {
        for byte in 0u8..=127 {
            let character = char::from(byte).to_string();
            assert_eq!(
                HOMOGRAPH_SAFE_ASCII.apply(&character),
                convert_to_homograph_safe_chars(&character),
                "character {byte:#04x}"
            );
        }
    }

    /// DPNS's `label` and `parentDomainName` patterns admit ASCII letters, digits and
    /// `-`; its normalized parent may also hold `.`.
    #[test]
    fn should_match_the_dpns_trigger_normalization_over_the_dpns_alphabets() {
        let alphabet: Vec<char> = ('a'..='z')
            .chain('A'..='Z')
            .chain('0'..='9')
            .chain(['-', '.'])
            .collect();
        for value in strings_up_to_three(&alphabet) {
            assert_eq!(
                HOMOGRAPH_SAFE_ASCII.apply(&value),
                convert_to_homograph_safe_chars(&value),
                "{value:?}"
            );
        }
        for value in [
            "Bob",
            "DASH",
            "dash",
            "Alice-In-Wonderland",
            "LoOoIiLl-0123456789",
            "a0b1c2-xyz-QRS-tuv-WXYZ-ok-oil-lol-io-Ol1ioL",
        ] {
            assert_eq!(
                HOMOGRAPH_SAFE_ASCII.apply(value),
                convert_to_homograph_safe_chars(value),
                "{value:?}"
            );
        }
    }

    #[test]
    fn should_normalize_letters_to_the_homograph_safe_form() {
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply("Bob"), "b0b");
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply("ALICE"), "a11ce");
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply("L0l-Io"), "101-10");
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply(""), "");
    }

    /// Outside ASCII the transform keeps every character, where the DPNS trigger's
    /// Unicode lowercasing would change some (the Kelvin sign lowercases to `k`,
    /// U+0130 to `i` followed by a combining dot).
    #[test]
    fn should_keep_every_character_outside_ascii() {
        for value in [
            "\u{212A}", "\u{0130}", "Ωmega", "bоb", "名前", "é", "ǅ", "🙂",
        ] {
            let normalized = HOMOGRAPH_SAFE_ASCII.apply(value);
            assert_eq!(
                normalized
                    .chars()
                    .filter(|c| !c.is_ascii())
                    .collect::<String>(),
                value.chars().filter(|c| !c.is_ascii()).collect::<String>(),
                "{value:?}"
            );
        }
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply("\u{212A}"), "\u{212A}");
        assert_ne!(convert_to_homograph_safe_chars("\u{212A}"), "\u{212A}");
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply("Ωmega"), "Ωmega");
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply("Olé"), "01é");
    }

    #[test]
    fn should_keep_the_byte_length_and_be_idempotent() {
        for value in ["Bob", "Ωmega-OIL", "名前Lo", "🙂io", ""] {
            let normalized = HOMOGRAPH_SAFE_ASCII.apply(value);
            assert_eq!(normalized.len(), value.len(), "{value:?}");
            assert_eq!(HOMOGRAPH_SAFE_ASCII.apply(&normalized), normalized);
        }
    }

    #[test]
    fn should_read_and_write_the_wire_name() {
        assert_eq!(
            NormalizationTransform::from_wire_name("homographSafeASCII"),
            Some(HOMOGRAPH_SAFE_ASCII)
        );
        assert_eq!(
            NormalizationTransform::from_wire_name("homographSafe"),
            None
        );
        assert_eq!(HOMOGRAPH_SAFE_ASCII.to_string(), "homographSafeASCII");
        let declaration = NormalizedFrom {
            property: "label".to_string(),
            transform: HOMOGRAPH_SAFE_ASCII,
        };
        assert_eq!(
            serde_json::to_value(&declaration).expect("serializes"),
            serde_json::json!({ "property": "label", "transform": "homographSafeASCII" })
        );
    }
}
