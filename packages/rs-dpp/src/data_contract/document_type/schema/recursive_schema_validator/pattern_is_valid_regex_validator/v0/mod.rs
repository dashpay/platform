use crate::consensus::basic::data_contract::IncompatibleRe2PatternError;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Value;
use regex::Regex;

#[inline(always)]
pub(super) fn pattern_is_valid_regex_validator_v0(
    path: &str,
    key: &str,
    _parent: &Value,
    value: &Value,
    result: &mut SimpleConsensusValidationResult,
) {
    if key == "pattern" {
        if let Some(pattern) = value.as_str() {
            // TODO: It doesn't make sense since Regex is not a RE2 engine
            //  and a doubt we use it in json schema library
            if let Err(err) = Regex::new(pattern) {
                result.add_error(IncompatibleRe2PatternError::new(
                    String::from(pattern),
                    path.to_string(),
                    err.to_string(),
                ));
            }
        } else {
            result.add_error(IncompatibleRe2PatternError::new(
                String::new(),
                path.to_string(),
                format!("{} is not a string", path),
            ));
        }
    }
}
