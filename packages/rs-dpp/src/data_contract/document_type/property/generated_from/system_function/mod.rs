//! The system functions a `generatedFrom` property is generated with: built
//! into the platform and named under `sys.`, so that functions a contract
//! brings later can be told apart by name. Each namespace under `sys.` is an
//! enum of its own in a submodule: `sys.stringTransformations` is
//! [`StringTransformation`], in [`string_transformations`].

pub mod string_transformations;

pub use string_transformations::StringTransformation;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A system function, by namespace.
///
/// A closed list of built-ins. Every node must compute exactly the same value,
/// so each function is defined without any table that could differ between
/// builds (Unicode case mappings change between Rust releases): the string
/// transformations change ASCII characters only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemFunction {
    /// `sys.stringTransformations.*`: one string to a string.
    StringTransformation(StringTransformation),
}

impl SystemFunction {
    /// Every function, in wire-name order.
    pub const ALL: [SystemFunction; StringTransformation::ALL.len()] = {
        let mut all = [SystemFunction::StringTransformation(StringTransformation::ALL[0]);
            StringTransformation::ALL.len()];
        let mut index = 0;
        while index < all.len() {
            all[index] = SystemFunction::StringTransformation(StringTransformation::ALL[index]);
            index += 1;
        }
        all
    };

    /// The wire name, the value of `generatedFrom.function`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            SystemFunction::StringTransformation(transformation) => transformation.as_str(),
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
            SystemFunction::StringTransformation(_) => 1,
        }
    }

    /// The value for `arguments`, the parameters' values in `params` order.
    /// `None` when their count is not the function's, which registration rules
    /// out. The platform writes this value into a document that leaves the
    /// property out, and compares a sent value with it.
    pub fn apply(&self, arguments: &[&str]) -> Option<String> {
        match (self, arguments) {
            (SystemFunction::StringTransformation(transformation), [source]) => {
                Some(transformation.apply(source))
            }
            _ => None,
        }
    }
}

impl fmt::Display for SystemFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Written as its wire name.
impl Serialize for SystemFunction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Read from its wire name; any other name is refused.
impl<'de> Deserialize<'de> for SystemFunction {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        SystemFunction::from_wire_name(&name)
            .ok_or_else(|| D::Error::custom(format!("unknown system function {name:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOMOGRAPH_SAFE_ASCII: SystemFunction =
        SystemFunction::StringTransformation(StringTransformation::HomographSafeAscii);

    #[test]
    fn should_apply_the_function_to_its_parameters() {
        assert_eq!(
            HOMOGRAPH_SAFE_ASCII.apply(&["Bob"]),
            Some("b0b".to_string())
        );
        assert_eq!(
            SystemFunction::StringTransformation(StringTransformation::SnakeCase)
                .apply(&["helloWorld"]),
            Some("hello_world".to_string())
        );
    }

    /// A call with another number of parameters than the function takes, which
    /// registration rules out, generates nothing.
    #[test]
    fn should_generate_nothing_for_another_number_of_parameters() {
        for function in SystemFunction::ALL {
            assert_eq!(function.parameter_count(), 1, "{function}");
            assert_eq!(function.apply(&[]), None, "{function}");
            assert_eq!(function.apply(&["a", "b"]), None, "{function}");
        }
    }

    #[test]
    fn should_list_every_string_transformation_in_wire_name_order() {
        let names: Vec<&str> = SystemFunction::ALL
            .iter()
            .map(SystemFunction::as_str)
            .collect();
        assert_eq!(
            names,
            [
                "sys.stringTransformations.camelCase",
                "sys.stringTransformations.capitalize",
                "sys.stringTransformations.homographSafeASCII",
                "sys.stringTransformations.lowercase",
                "sys.stringTransformations.snakeCase",
                "sys.stringTransformations.uppercase",
            ]
        );
    }

    #[test]
    fn should_read_and_write_the_wire_name() {
        for function in SystemFunction::ALL {
            assert_eq!(
                SystemFunction::from_wire_name(function.as_str()),
                Some(function)
            );
            assert_eq!(function.to_string(), function.as_str());
            let written = serde_json::to_value(function).expect("serializes");
            assert_eq!(written, serde_json::json!(function.as_str()));
            assert_eq!(
                serde_json::from_value::<SystemFunction>(written).expect("deserializes"),
                function
            );
        }
        assert_eq!(SystemFunction::from_wire_name("homographSafeASCII"), None);
        assert_eq!(
            SystemFunction::from_wire_name("sys.stringTransformations.lowerCase"),
            None
        );
        assert!(serde_json::from_value::<SystemFunction>(serde_json::json!("sys.nope")).is_err());
    }

    /// Meta-schema v3 lists every system function the parser knows, so a name
    /// it admits always parses and none the parser knows is refused by it.
    #[test]
    fn should_list_every_function_in_the_meta_schema() {
        let meta_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../schema/meta_schemas/document/v3/document-meta.json"
        ))
        .expect("the meta-schema is JSON");
        let listed = meta_schema
            .pointer("/$defs/documentSchema/properties/generatedFrom/properties/function/enum")
            .and_then(|names| names.as_array())
            .expect("the meta-schema lists the functions");
        let names: Vec<&str> = SystemFunction::ALL
            .iter()
            .map(SystemFunction::as_str)
            .collect();
        assert_eq!(
            listed
                .iter()
                .map(|name| name.as_str().expect("a name"))
                .collect::<Vec<_>>(),
            names
        );
    }
}
