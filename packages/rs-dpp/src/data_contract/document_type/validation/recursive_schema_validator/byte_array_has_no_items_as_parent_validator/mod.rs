use crate::validation::SimpleConsensusValidationResult;
use platform_value::Value;
use platform_version::version::PlatformVersion;

mod v0;

pub fn byte_array_has_no_items_as_parent_validator(
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
        .byte_array_has_no_items_as_parent_validator
    {
        0 => v0::byte_array_has_no_items_as_parent_validator_v0(path, key, parent, value, result),
        version => unimplemented!("byte_array_has_no_items_as_parent_validator_v{}", version),
    }
}
