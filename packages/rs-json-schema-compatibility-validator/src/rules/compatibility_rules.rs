#[cfg(any(test, feature = "examples"))]
use crate::change::JsonSchemaChange;
use crate::error::Error;
use json_patch::ReplaceOperation;
use serde_json::Value;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// Type alias for an optional callback function that is called when [ReplaceOperation] is applied to a schema.
/// It takes the original schema as a reference to a [Value] and a [ReplaceOperation],
/// and returns a `Result<bool, Error>`. `True` means that the operation is compatible,
/// `False` means that the operation is incompatible.
// This function is wrapped in an `Arc` for lazy evaluation.
pub type IsReplacementAllowedCallback =
    Option<Arc<dyn Fn(&Value, &ReplaceOperation) -> Result<bool, Error> + Send + Sync>>;

/// Struct representing a compatibility rules in a JSON schema, such as allowing
/// adding, removing, and replacing of the schema elements. It also optionally contains inner structure rule,
/// and a list of examples.
#[derive(Clone)]
pub struct CompatibilityRules {
    /// Boolean indicating whether adding is allowed.
    pub allow_addition: bool,
    /// Boolean indicating whether removing is allowed.
    pub allow_removal: bool,
    /// Compatibility for replacing is often based on the previous state and new value,
    /// so [IsReplacementAllowedCallback] is used to define this dynamic logic.
    /// The callback is optional because replacing is impossible for some schema elements
    /// due to inner structure.
    pub allow_replacement_callback: IsReplacementAllowedCallback,
    /// Optional number of levels to an inner subschema in case if the element
    /// contains a subschema inside.
    /// When the next subschema is reached, the new next compatibility rules will be applied
    /// based the rule finding algorithm.
    pub subschema_levels_depth: Option<usize>,
    /// Compatibility rules for inner structure (until the next subschema defined in `levels_to_subschema`).
    pub inner: Option<Box<CompatibilityRules>>,
    /// Examples (vector of [CompatibilityRuleExample]) of the compatibility rules.
    /// Only available when testing or when the `examples` feature is enabled.
    #[cfg(any(test, feature = "examples"))]
    pub examples: Vec<CompatibilityRuleExample>,
}

impl Debug for CompatibilityRules {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("CompatibilityRules");

        let replace_callback = if self.allow_replacement_callback.is_some() {
            Some("ReplaceCallback")
        } else {
            None
        };

        debug
            .field("allow_adding", &self.allow_addition)
            .field("allow_removing", &self.allow_removal)
            .field("allow_replacing", &replace_callback)
            .field("subschema_levels_depth", &self.subschema_levels_depth)
            .field("inner", &self.inner);

        #[cfg(any(test, feature = "examples"))]
        let debug = debug.field("examples", &self.examples);

        debug.finish()
    }
}

impl PartialEq for CompatibilityRules {
    fn eq(&self, other: &Self) -> bool {
        #[allow(unused_mut, unused_assignments)]
        let mut examples = true;

        #[cfg(any(test, feature = "examples"))]
        {
            examples = self.examples == other.examples;
        }

        self.allow_addition == other.allow_addition
            && self.allow_removal == other.allow_removal
            && self.allow_replacement_callback.is_some()
                == other.allow_replacement_callback.is_some()
            && self.inner == other.inner
            && self.subschema_levels_depth == other.subschema_levels_depth
            && examples
    }
}

/// Struct representing an example of a compatibility rule.
/// Only available when testing or when the "examples" feature is enabled.
#[cfg(any(test, feature = "examples"))]
#[derive(Debug, PartialEq, Clone)]
pub struct CompatibilityRuleExample {
    /// The original JSON schema.
    pub original_schema: Value,
    /// The new JSON schema.
    pub new_schema: Value,
    /// Incompatible [JsonSchemaChange] in the JSON schema if it has a place.
    /// `None` if the change is compatible.
    pub incompatible_change: Option<JsonSchemaChange>,
}

/// Implementation of the [From] trait for [CompatibilityRuleExample]. Allows for creating a [CompatibilityRuleExample]
/// from a tuple of values, and an optional [JsonSchemaChange] in case if we expect an incompatible change.
#[cfg(any(test, feature = "examples"))]
impl From<(Value, Value, Option<JsonSchemaChange>)> for CompatibilityRuleExample {
    fn from(
        (original_schema, new_schema, incompatible_change): (
            Value,
            Value,
            Option<JsonSchemaChange>,
        ),
    ) -> Self {
        Self {
            original_schema,
            new_schema,
            incompatible_change,
        }
    }
}
