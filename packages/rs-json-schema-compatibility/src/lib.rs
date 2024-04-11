mod change;
mod error;
mod keyword;
mod validate;

pub use crate::change::JsonSchemaChange;
pub use crate::keyword::{KeywordRule, KeywordRuleExample, ReplaceCallback, KEYWORD_RULES};
pub use json_patch::{AddOperation, RemoveOperation, ReplaceOperation};
pub use validate::{validate_schemas_compatibility, CompatibilityValidationResult};
