//! The `generatedFrom` declaration of a property: the platform generates the
//! property's value with a function of other properties of the same document,
//! such as a name folded for case-insensitive, homograph-resistant uniqueness.
//!
//! The platform computes the property when a created or replaced document
//! leaves it out and supplies every parameter, and checks it when the document
//! supplies it. The functions are a closed list of built-ins named under
//! `sys.`. A function never refuses a value: which characters a parameter may
//! hold is the job of that parameter's own `pattern`, and the generated
//! property needs no pattern of its own.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The `generatedFrom` declaration of a property.
///
/// Declared as
/// `"generatedFrom": { "function": "sys.stringTransformations.homographSafeASCII", "params": ["<dotted property path>"] }`
/// on a string property (meta-schema v3, protocol version 14). The declaring
/// property must hold what `function` returns for the values of `params`, and
/// be absent exactly when a parameter is. The parser checks at contract
/// registration that `params` holds as many parameters as the function takes,
/// each another property of the same document type of the kind the function
/// reads, none transient, none generated itself, and each inside every object
/// that holds the declaring property, so a document supplying the parameters
/// always has somewhere to put the generated value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedFrom {
    /// The function that generates the value.
    pub function: GenerationFunction,
    /// What the function is applied to, in order.
    pub params: Vec<GenerationParam>,
}

impl GeneratedFrom {
    /// The property paths the parameters read, in order.
    pub fn property_params(&self) -> impl Iterator<Item = &str> {
        self.params.iter().map(|param| match param {
            GenerationParam::Property(path) => path.as_str(),
        })
    }

    /// The value the declaring property must hold for the parameters' values, in
    /// `params` order. `None` when their count is not the function's, which
    /// registration rules out.
    pub fn generate(&self, arguments: &[&str]) -> Option<String> {
        self.function.apply(arguments)
    }

    /// Whether `value` is what the function returns for `arguments`, compared
    /// character by character without building the generated string.
    pub fn is_generated_value(&self, arguments: &[&str], value: &str) -> bool {
        self.function.is_applied(arguments, value)
    }

    /// The parameters as the schema spells them, joined for a message.
    pub fn params_description(&self) -> String {
        self.property_params().collect::<Vec<_>>().join(", ")
    }
}

/// One parameter of a [`GeneratedFrom`] function.
///
/// Only properties of the same document for now, written as a bare dotted
/// path. The grammar leaves room for literals (`{ "const": ... }`), system
/// values (`"$ownerId"`) and nested calls, which a later protocol version can
/// add without changing what parses today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GenerationParam {
    /// The dotted path of a property of the same document type.
    Property(String),
}

/// A function a [`GeneratedFrom`] property is generated with.
///
/// A closed list of built-ins, named under `sys.` so that functions a
/// contract brings later can be told apart by name. Every node must compute
/// exactly the same value, so each function is defined without any table that
/// could differ between builds (Unicode case mappings change between Rust
/// releases).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationFunction {
    /// DPNS's label normalization over ASCII, of one string: `A` to `Z`
    /// become lowercase, then `o` becomes `0` and `i` and `l` become `1`.
    /// Every other character, ASCII or not, is kept as it is, so the value
    /// keeps its length. On an ASCII value this is exactly
    /// `convert_to_homograph_safe_chars`; it resists homographs only when the
    /// parameter's `pattern` restricts it to ASCII, as DPNS's does.
    #[serde(rename = "sys.stringTransformations.homographSafeASCII")]
    HomographSafeAscii,
}

impl GenerationFunction {
    /// Every function, in wire-name order.
    pub const ALL: [GenerationFunction; 1] = [GenerationFunction::HomographSafeAscii];

    /// The wire name, the value of `generatedFrom.function`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            GenerationFunction::HomographSafeAscii => {
                "sys.stringTransformations.homographSafeASCII"
            }
        }
    }

    /// The function a wire name names, `None` for any other name.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|function| function.as_str() == name)
    }

    /// How many parameters the function takes. Every one is a string.
    pub const fn parameter_count(&self) -> usize {
        match self {
            GenerationFunction::HomographSafeAscii => 1,
        }
    }

    /// What one character of the parameter becomes.
    fn apply_to_character(&self, character: char) -> char {
        match self {
            GenerationFunction::HomographSafeAscii => match character.to_ascii_lowercase() {
                'o' => '0',
                'i' | 'l' => '1',
                other => other,
            },
        }
    }

    /// The value for `arguments`, `None` when their count is not the
    /// function's.
    pub fn apply(&self, arguments: &[&str]) -> Option<String> {
        match (self, arguments) {
            (GenerationFunction::HomographSafeAscii, [source]) => Some(
                source
                    .chars()
                    .map(|character| self.apply_to_character(character))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// Whether `value` is the value for `arguments`, without building it.
    /// `false` when their count is not the function's.
    pub fn is_applied(&self, arguments: &[&str], value: &str) -> bool {
        match (self, arguments) {
            (GenerationFunction::HomographSafeAscii, [source]) => value.chars().eq(source
                .chars()
                .map(|character| self.apply_to_character(character))),
            _ => false,
        }
    }
}

impl fmt::Display for GenerationFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::strings::convert_to_homograph_safe_chars;

    const HOMOGRAPH_SAFE_ASCII: GenerationFunction = GenerationFunction::HomographSafeAscii;

    fn apply(value: &str) -> String {
        HOMOGRAPH_SAFE_ASCII
            .apply(&[value])
            .expect("one parameter is the function's count")
    }

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
                apply(&character),
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
                apply(&value),
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
                apply(value),
                convert_to_homograph_safe_chars(value),
                "{value:?}"
            );
        }
    }

    #[test]
    fn should_generate_the_homograph_safe_form() {
        assert_eq!(apply("Bob"), "b0b");
        assert_eq!(apply("ALICE"), "a11ce");
        assert_eq!(apply("L0l-Io"), "101-10");
        assert_eq!(apply(""), "");
    }

    /// Outside ASCII the function keeps every character, where the DPNS trigger's
    /// Unicode lowercasing would change some (the Kelvin sign lowercases to `k`,
    /// U+0130 to `i` followed by a combining dot).
    #[test]
    fn should_keep_every_character_outside_ascii() {
        for value in [
            "\u{212A}", "\u{0130}", "Ωmega", "bоb", "名前", "é", "ǅ", "🙂",
        ] {
            let generated = apply(value);
            assert_eq!(
                generated
                    .chars()
                    .filter(|c| !c.is_ascii())
                    .collect::<String>(),
                value.chars().filter(|c| !c.is_ascii()).collect::<String>(),
                "{value:?}"
            );
        }
        assert_eq!(apply("\u{212A}"), "\u{212A}");
        assert_ne!(convert_to_homograph_safe_chars("\u{212A}"), "\u{212A}");
        assert_eq!(apply("Ωmega"), "Ωmega");
        assert_eq!(apply("Olé"), "01é");
    }

    #[test]
    fn should_keep_the_byte_length_and_be_idempotent() {
        for value in ["Bob", "Ωmega-OIL", "名前Lo", "🙂io", ""] {
            let generated = apply(value);
            assert_eq!(generated.len(), value.len(), "{value:?}");
            assert_eq!(apply(&generated), generated);
        }
    }

    #[test]
    fn should_compare_a_value_with_the_generated_one_without_building_it() {
        for (source, value, expected) in [
            ("Bob", "b0b", true),
            ("Bob", "bob", false),
            ("Bob", "b0", false),
            ("Bob", "b0bb", false),
            ("", "", true),
            ("Olé", "01é", true),
        ] {
            assert_eq!(
                HOMOGRAPH_SAFE_ASCII.is_applied(&[source], value),
                expected,
                "{source:?} / {value:?}"
            );
            assert_eq!(apply(source) == value, expected);
        }
    }

    /// A call with another number of parameters than the function takes, which
    /// registration rules out, generates nothing and matches nothing.
    #[test]
    fn should_generate_nothing_for_another_number_of_parameters() {
        assert_eq!(HOMOGRAPH_SAFE_ASCII.parameter_count(), 1);
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply(&[]), None);
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply(&["a", "b"]), None);
        assert!(!HOMOGRAPH_SAFE_ASCII.is_applied(&[], ""));
        assert!(!HOMOGRAPH_SAFE_ASCII.is_applied(&["a", "b"], "a"));
    }

    #[test]
    fn should_read_and_write_the_wire_form() {
        assert_eq!(
            GenerationFunction::from_wire_name("sys.stringTransformations.homographSafeASCII"),
            Some(HOMOGRAPH_SAFE_ASCII)
        );
        assert_eq!(
            GenerationFunction::from_wire_name("homographSafeASCII"),
            None
        );
        assert_eq!(
            HOMOGRAPH_SAFE_ASCII.to_string(),
            "sys.stringTransformations.homographSafeASCII"
        );
        let declaration = GeneratedFrom {
            function: HOMOGRAPH_SAFE_ASCII,
            params: vec![GenerationParam::Property("label".to_string())],
        };
        assert_eq!(declaration.params_description(), "label");
        assert_eq!(
            serde_json::to_value(&declaration).expect("serializes"),
            serde_json::json!({
                "function": "sys.stringTransformations.homographSafeASCII",
                "params": ["label"]
            })
        );
    }
}
