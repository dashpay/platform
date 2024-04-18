mod compatibility_rules;
mod rule_set;
mod value;

#[cfg(any(test, feature = "examples"))]
pub use compatibility_rules::CompatibilityRuleExample;
pub use compatibility_rules::{CompatibilityRules, IsReplacementAllowedCallback};
pub use rule_set::KEYWORD_COMPATIBILITY_RULES;
