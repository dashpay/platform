//! `sys.stringTransformations`: system functions from one string to a string.
//!
//! Every transformation changes ASCII characters only and keeps every other
//! character as it is: Unicode case mappings change between releases of the
//! standard library, and two nodes must never generate different values.

/// A `sys.stringTransformations` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringTransformation {
    /// `sys.stringTransformations.camelCase`: [`camel_case`].
    CamelCase,
    /// `sys.stringTransformations.capitalize`: [`capitalize`].
    Capitalize,
    /// `sys.stringTransformations.homographSafeASCII`: [`homograph_safe_ascii`].
    HomographSafeAscii,
    /// `sys.stringTransformations.lowercase`: [`lowercase`].
    Lowercase,
    /// `sys.stringTransformations.snakeCase`: [`snake_case`].
    SnakeCase,
    /// `sys.stringTransformations.uppercase`: [`uppercase`].
    Uppercase,
}

impl StringTransformation {
    /// Every string transformation, in wire-name order.
    pub const ALL: [StringTransformation; 6] = [
        StringTransformation::CamelCase,
        StringTransformation::Capitalize,
        StringTransformation::HomographSafeAscii,
        StringTransformation::Lowercase,
        StringTransformation::SnakeCase,
        StringTransformation::Uppercase,
    ];

    /// The wire name, the value of `generatedFrom.function`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            StringTransformation::CamelCase => "sys.stringTransformations.camelCase",
            StringTransformation::Capitalize => "sys.stringTransformations.capitalize",
            StringTransformation::HomographSafeAscii => {
                "sys.stringTransformations.homographSafeASCII"
            }
            StringTransformation::Lowercase => "sys.stringTransformations.lowercase",
            StringTransformation::SnakeCase => "sys.stringTransformations.snakeCase",
            StringTransformation::Uppercase => "sys.stringTransformations.uppercase",
        }
    }

    /// The transformation of `source`.
    pub fn apply(&self, source: &str) -> String {
        match self {
            StringTransformation::CamelCase => camel_case(source),
            StringTransformation::Capitalize => capitalize(source),
            StringTransformation::HomographSafeAscii => homograph_safe_ascii(source),
            StringTransformation::Lowercase => lowercase(source),
            StringTransformation::SnakeCase => snake_case(source),
            StringTransformation::Uppercase => uppercase(source),
        }
    }
}

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

/// `sys.stringTransformations.lowercase`: `A` to `Z` become `a` to `z`.
pub fn lowercase(source: &str) -> String {
    source.to_ascii_lowercase()
}

/// `sys.stringTransformations.uppercase`: `a` to `z` become `A` to `Z`.
pub fn uppercase(source: &str) -> String {
    source.to_ascii_uppercase()
}

/// `sys.stringTransformations.capitalize`: the first character uppercase and
/// every other lowercase (`hELLO wORLD` becomes `Hello world`).
pub fn capitalize(source: &str) -> String {
    let mut characters = source.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    let mut capitalized = String::with_capacity(source.len());
    capitalized.push(first.to_ascii_uppercase());
    capitalized.push_str(&characters.as_str().to_ascii_lowercase());
    capitalized
}

/// `sys.stringTransformations.camelCase`: the [`words`] joined, the first
/// lowercase and every later one capitalized (`Hello world`, `hello_world` and
/// `HelloWorld` all become `helloWorld`).
pub fn camel_case(source: &str) -> String {
    let mut camel = String::with_capacity(source.len());
    for (index, word) in words(source).into_iter().enumerate() {
        if index == 0 {
            camel.push_str(&lowercase(word));
        } else {
            camel.push_str(&capitalize(word));
        }
    }
    camel
}

/// `sys.stringTransformations.snakeCase`: the [`words`] lowercase, joined with
/// `_` (`Hello world`, `helloWorld` and `hello-world` all become
/// `hello_world`).
pub fn snake_case(source: &str) -> String {
    words(source)
        .into_iter()
        .map(lowercase)
        .collect::<Vec<_>>()
        .join("_")
}

/// The words of `source`, as camelCase and snakeCase split it. Every ASCII
/// character that is neither a letter nor a digit separates words and is
/// dropped. A word also ends before an ASCII uppercase letter that follows any
/// other character than an ASCII uppercase letter (`helloWorld` is `hello`,
/// `World`), and before one that follows another and is followed by an ASCII
/// lowercase letter (`XMLHttp` is `XML`, `Http`). Characters outside ASCII are
/// word characters: they never separate words, and never change case. The
/// rules make camelCase and snakeCase idempotent: splitting their output gives
/// back the same words.
pub fn words(source: &str) -> Vec<&str> {
    let characters: Vec<(usize, char)> = source.char_indices().collect();
    let mut words = Vec::new();
    let mut word_start: Option<usize> = None;
    for (position, &(index, character)) in characters.iter().enumerate() {
        if character.is_ascii() && !character.is_ascii_alphanumeric() {
            if let Some(start) = word_start.take() {
                words.push(&source[start..index]);
            }
            continue;
        }
        let Some(start) = word_start else {
            word_start = Some(index);
            continue;
        };
        // The word holds the previous character: a separator would have ended it
        let previous = characters[position - 1].1;
        let next = characters.get(position + 1).map(|&(_, next)| next);
        let starts_a_word = character.is_ascii_uppercase()
            && (!previous.is_ascii_uppercase()
                || next.is_some_and(|next| next.is_ascii_lowercase()));
        if starts_a_word {
            words.push(&source[start..index]);
            word_start = Some(index);
        }
    }
    if let Some(start) = word_start {
        words.push(&source[start..]);
    }
    words
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

    /// Strings that exercise every splitting rule, plus characters outside
    /// ASCII.
    const SAMPLES: &[&str] = &[
        "",
        "hello",
        "Hello World",
        "hello_world",
        "hello-world",
        "helloWorld",
        "HelloWorld",
        "XMLHttpRequest",
        "version2Beta",
        "  leading and trailing  ",
        "__many___separators--",
        "ALL CAPS",
        "Olé Señor",
        "oléSeñor",
        "名前Lo",
        "aBCd",
        "hello 2World",
    ];

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

    #[test]
    fn should_change_the_case_of_ascii_letters() {
        assert_eq!(lowercase("Hello World 42"), "hello world 42");
        assert_eq!(uppercase("Hello World 42"), "HELLO WORLD 42");
        assert_eq!(capitalize("hELLO wORLD"), "Hello world");
        assert_eq!(capitalize("bob"), "Bob");
        assert_eq!(capitalize(" bob"), " bob");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn should_split_words_at_separators_and_case_changes() {
        assert_eq!(words("Hello World"), ["Hello", "World"]);
        assert_eq!(
            words("hello_world-again.now"),
            ["hello", "world", "again", "now"]
        );
        assert_eq!(words("helloWorld"), ["hello", "World"]);
        assert_eq!(words("XMLHttpRequest"), ["XML", "Http", "Request"]);
        assert_eq!(words("version2Beta"), ["version2", "Beta"]);
        assert_eq!(words("__many___separators--"), ["many", "separators"]);
        assert_eq!(words("ALL CAPS"), ["ALL", "CAPS"]);
        assert_eq!(words("Olé Señor"), ["Olé", "Señor"]);
        assert_eq!(words("éB"), ["é", "B"]);
        assert_eq!(words("aBCd"), ["a", "B", "Cd"]);
        assert!(words("").is_empty());
        assert!(words("-_ .").is_empty());
    }

    #[test]
    fn should_generate_camel_case_and_snake_case() {
        for (source, camel, snake) in [
            ("Hello World", "helloWorld", "hello_world"),
            ("hello_world", "helloWorld", "hello_world"),
            ("hello-world", "helloWorld", "hello_world"),
            ("helloWorld", "helloWorld", "hello_world"),
            ("HelloWorld", "helloWorld", "hello_world"),
            ("XMLHttpRequest", "xmlHttpRequest", "xml_http_request"),
            ("version2Beta", "version2Beta", "version2_beta"),
            (
                "  leading and trailing  ",
                "leadingAndTrailing",
                "leading_and_trailing",
            ),
            ("ALL CAPS", "allCaps", "all_caps"),
            ("Olé Señor", "oléSeñor", "olé_señor"),
            ("", "", ""),
        ] {
            assert_eq!(camel_case(source), camel, "{source:?}");
            assert_eq!(snake_case(source), snake, "{source:?}");
        }
    }

    /// Characters outside ASCII are never changed, where a Unicode case mapping
    /// would change some (the Kelvin sign lowercases to `k`, U+0130 to `i`
    /// followed by a combining dot).
    #[test]
    fn should_keep_every_character_outside_ascii() {
        let non_ascii = |value: &str| value.chars().filter(|c| !c.is_ascii()).collect::<String>();
        for value in [
            "\u{212A}",
            "\u{0130}",
            "Ωmega",
            "bоb",
            "名前",
            "é",
            "ǅ",
            "🙂",
            "Olé Señor",
        ] {
            for transformation in StringTransformation::ALL {
                assert_eq!(
                    non_ascii(&transformation.apply(value)),
                    non_ascii(value),
                    "{} of {value:?}",
                    transformation.as_str()
                );
            }
        }
        assert_eq!(homograph_safe_ascii("\u{212A}"), "\u{212A}");
        assert_ne!(convert_to_homograph_safe_chars("\u{212A}"), "\u{212A}");
        assert_eq!(uppercase("ω"), "ω");
        assert_eq!(homograph_safe_ascii("Olé"), "01é");
    }

    /// Every transformation applied to its own output changes nothing more.
    #[test]
    fn should_be_idempotent() {
        for transformation in StringTransformation::ALL {
            for value in SAMPLES {
                let once = transformation.apply(value);
                assert_eq!(
                    transformation.apply(&once),
                    once,
                    "{} of {value:?}",
                    transformation.as_str()
                );
            }
        }
    }

    /// The case changes keep the byte length, camelCase never grows a value, and
    /// snakeCase grows it by at most one `_` per character.
    #[test]
    fn should_bound_the_length_of_the_generated_value() {
        for value in SAMPLES {
            for transformation in [
                StringTransformation::Capitalize,
                StringTransformation::HomographSafeAscii,
                StringTransformation::Lowercase,
                StringTransformation::Uppercase,
            ] {
                assert_eq!(transformation.apply(value).len(), value.len(), "{value:?}");
            }
            assert!(camel_case(value).len() <= value.len(), "{value:?}");
            assert!(snake_case(value).len() <= 2 * value.len(), "{value:?}");
        }
    }
}
