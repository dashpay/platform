//! The `generatedFrom` declaration of a property: the platform generates the
//! property's value with a function of other properties of the same document,
//! such as a name folded for case-insensitive, homograph-resistant uniqueness.
//!
//! The platform computes the property when a created or replaced document
//! leaves it out and supplies every parameter, and checks it when the document
//! supplies it. The functions are a closed list of system functions named
//! under `sys.` ([`system_function`]). A function never refuses a value: which
//! characters a parameter may hold is the job of that parameter's own
//! `pattern`, and the generated property needs no pattern of its own.

pub mod system_function;

pub use system_function::{HashFunction, StringTransformation, SystemFunction};

use serde::{Deserialize, Serialize};

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
    pub function: SystemFunction,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_and_write_the_wire_form() {
        let declaration = GeneratedFrom {
            function: SystemFunction::StringTransformation(
                StringTransformation::HomographSafeAscii,
            ),
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
