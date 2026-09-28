//! The built-in functions a `generatedFrom` property is generated with, named
//! under `sys.` so that functions a contract brings later can be told apart by
//! name. Each namespace under `sys.` is a submodule holding its functions:
//! `sys.stringTransformations` is [`string_transformations`].

pub mod string_transformations;

use serde::{Deserialize, Serialize};
use std::fmt;

/// A function a `generatedFrom` property is generated with.
///
/// A closed list of built-ins. Every node must compute exactly the same value,
/// so each function is defined without any table that could differ between
/// builds (Unicode case mappings change between Rust releases).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationFunction {
    /// `sys.stringTransformations.homographSafeASCII`, of one string:
    /// [`string_transformations::homograph_safe_ascii`].
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

    /// The value for `arguments`, the parameters' values in `params` order.
    /// `None` when their count is not the function's, which registration rules
    /// out. The platform writes this value into a document that leaves the
    /// property out, and compares a sent value with it.
    pub fn apply(&self, arguments: &[&str]) -> Option<String> {
        match (self, arguments) {
            (GenerationFunction::HomographSafeAscii, [source]) => {
                Some(string_transformations::homograph_safe_ascii(source))
            }
            _ => None,
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

    const HOMOGRAPH_SAFE_ASCII: GenerationFunction = GenerationFunction::HomographSafeAscii;

    #[test]
    fn should_apply_the_function_to_its_parameters() {
        assert_eq!(
            HOMOGRAPH_SAFE_ASCII.apply(&["Bob"]),
            Some("b0b".to_string())
        );
    }

    /// A call with another number of parameters than the function takes, which
    /// registration rules out, generates nothing.
    #[test]
    fn should_generate_nothing_for_another_number_of_parameters() {
        assert_eq!(HOMOGRAPH_SAFE_ASCII.parameter_count(), 1);
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply(&[]), None);
        assert_eq!(HOMOGRAPH_SAFE_ASCII.apply(&["a", "b"]), None);
    }

    #[test]
    fn should_read_and_write_the_wire_name() {
        for function in GenerationFunction::ALL {
            assert_eq!(
                GenerationFunction::from_wire_name(function.as_str()),
                Some(function)
            );
            assert_eq!(function.to_string(), function.as_str());
            assert_eq!(
                serde_json::to_value(function).expect("serializes"),
                serde_json::json!(function.as_str())
            );
        }
        assert_eq!(
            HOMOGRAPH_SAFE_ASCII.as_str(),
            "sys.stringTransformations.homographSafeASCII"
        );
        assert_eq!(
            GenerationFunction::from_wire_name("homographSafeASCII"),
            None
        );
    }
}
