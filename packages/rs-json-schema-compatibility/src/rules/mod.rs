mod compatibility_rules;
mod rule_set;
mod value;

pub use compatibility_rules::{
    CompatibilityRuleExample, CompatibilityRules, IsReplacementAllowedCallback,
};
pub use rule_set::KEYWORD_COMPATIBILITY_RULES;
