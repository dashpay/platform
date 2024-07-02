mod compatibility_rules;
mod ruleset;
mod value;

#[cfg(any(test, feature = "examples"))]
pub use compatibility_rules::CompatibilityRuleExample;
pub use compatibility_rules::{CompatibilityRules, IsReplacementAllowedCallback};
pub use ruleset::CompatibilityRulesCollection;
pub use ruleset::KEYWORD_COMPATIBILITY_RULES;
