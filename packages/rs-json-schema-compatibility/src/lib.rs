mod change;
mod error;
mod rules;
mod validate;

pub use crate::change::JsonSchemaChange;
pub use crate::rules::{
    CompatibilityRuleExample, CompatibilityRules, IsReplacementAllowedCallback,
    KEYWORD_COMPATIBILITY_RULES,
};
pub use json_patch::{AddOperation, RemoveOperation, ReplaceOperation};
pub use validate::{validate_schemas_compatibility, CompatibilityValidationResult};
