#[cfg(any(test, feature = "examples"))]
use crate::change::JsonSchemaChange;
use crate::error::Error;
use json_patch::ReplaceOperation;
use serde_json::Value;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

pub type ReplaceCallback =
    Option<Arc<dyn Fn(&Value, &ReplaceOperation) -> Result<bool, Error> + Send + Sync>>;

pub struct KeywordRule {
    pub allow_adding: bool,
    pub allow_removing: bool,
    pub allow_replacing: ReplaceCallback,
    pub levels_to_subschema: Option<usize>,
    pub inner: Option<Box<KeywordRule>>,
    #[cfg(any(test, feature = "examples"))]
    pub examples: Vec<KeywordRuleExample>,
}

impl Debug for KeywordRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("KeywordRule");

        let replace_callback = if self.allow_replacing.is_some() {
            Some("ReplaceCallback")
        } else {
            None
        };

        debug
            .field("allow_adding", &self.allow_adding)
            .field("allow_removing", &self.allow_removing)
            .field("allow_replacing", &replace_callback)
            .field("levels_to_subschema", &self.levels_to_subschema)
            .field("inner", &self.inner);

        #[cfg(any(test, feature = "examples"))]
        let debug = debug.field("examples", &self.examples);

        debug.finish()
    }
}

impl PartialEq for KeywordRule {
    fn eq(&self, other: &Self) -> bool {
        #[allow(unused_mut, unused_assignments)]
        let mut examples = true;

        #[cfg(any(test, feature = "examples"))]
        {
            examples = self.examples == other.examples;
        }

        self.allow_adding == other.allow_adding
            && self.allow_removing == other.allow_removing
            && self.allow_replacing.is_some() == other.allow_replacing.is_some()
            && self.inner == other.inner
            && self.levels_to_subschema == other.levels_to_subschema
            && examples
    }
}

#[cfg(any(test, feature = "examples"))]
#[derive(Debug, PartialEq)]
pub struct KeywordRuleExample {
    pub original_schema: Value,
    pub new_schema: Value,
    pub incompatible_change: Option<JsonSchemaChange>,
}

#[cfg(any(test, feature = "examples"))]
impl From<(Value, Value, Option<JsonSchemaChange>)> for KeywordRuleExample {
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
