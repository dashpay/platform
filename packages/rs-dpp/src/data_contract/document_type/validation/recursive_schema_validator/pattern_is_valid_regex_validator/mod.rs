use crate::validation::SimpleConsensusValidationResult;
use platform_value::Value;
use platform_version::version::PlatformVersion;

mod v0;

pub fn pattern_is_valid_regex_validator(
    path: &str,
    key: &str,
    parent: &Value,
    value: &Value,
    result: &mut SimpleConsensusValidationResult,
    platform_version: &PlatformVersion,
) {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .validation_versions
        .recursive_schema_validator_versions
        .pattern_is_valid_regex_validator
    {
        0 => v0::pattern_is_valid_regex_validator_v0(path, key, parent, value, result),
        version => unimplemented!("pattern_is_valid_regex_validator_v{}", version),
    }
}
