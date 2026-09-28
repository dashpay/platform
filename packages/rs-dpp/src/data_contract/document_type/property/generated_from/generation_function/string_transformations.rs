//! `sys.stringTransformations`: built-in functions from strings to a string.

/// `sys.stringTransformations.homographSafeASCII`: DPNS's label normalization
/// over ASCII. `A` to `Z` become lowercase, then `o` becomes `0` and `i` and
/// `l` become `1`. Every other character, ASCII or not, is kept as it is, so
/// the value keeps its length.
///
/// On an ASCII value this is exactly `convert_to_homograph_safe_chars`; it
/// resists homographs only when the parameter's `pattern` restricts it to
/// ASCII, as DPNS's does.
pub fn homograph_safe_ascii(source: &str) -> String {
    source
        .chars()
        .map(|character| match character.to_ascii_lowercase() {
            'o' => '0',
            'i' | 'l' => '1',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::strings::convert_to_homograph_safe_chars;

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
                homograph_safe_ascii(&character),
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
                homograph_safe_ascii(&value),
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
                homograph_safe_ascii(value),
                convert_to_homograph_safe_chars(value),
                "{value:?}"
            );
        }
    }

    #[test]
    fn should_generate_the_homograph_safe_form() {
        assert_eq!(homograph_safe_ascii("Bob"), "b0b");
        assert_eq!(homograph_safe_ascii("ALICE"), "a11ce");
        assert_eq!(homograph_safe_ascii("L0l-Io"), "101-10");
        assert_eq!(homograph_safe_ascii(""), "");
    }

    /// Outside ASCII the function keeps every character, where the DPNS trigger's
    /// Unicode lowercasing would change some (the Kelvin sign lowercases to `k`,
    /// U+0130 to `i` followed by a combining dot).
    #[test]
    fn should_keep_every_character_outside_ascii() {
        for value in [
            "\u{212A}", "\u{0130}", "Ωmega", "bоb", "名前", "é", "ǅ", "🙂",
        ] {
            let generated = homograph_safe_ascii(value);
            assert_eq!(
                generated
                    .chars()
                    .filter(|c| !c.is_ascii())
                    .collect::<String>(),
                value.chars().filter(|c| !c.is_ascii()).collect::<String>(),
                "{value:?}"
            );
        }
        assert_eq!(homograph_safe_ascii("\u{212A}"), "\u{212A}");
        assert_ne!(convert_to_homograph_safe_chars("\u{212A}"), "\u{212A}");
        assert_eq!(homograph_safe_ascii("Ωmega"), "Ωmega");
        assert_eq!(homograph_safe_ascii("Olé"), "01é");
    }

    #[test]
    fn should_keep_the_byte_length_and_be_idempotent() {
        for value in ["Bob", "Ωmega-OIL", "名前Lo", "🙂io", ""] {
            let generated = homograph_safe_ascii(value);
            assert_eq!(generated.len(), value.len(), "{value:?}");
            assert_eq!(homograph_safe_ascii(&generated), generated);
        }
    }
}
