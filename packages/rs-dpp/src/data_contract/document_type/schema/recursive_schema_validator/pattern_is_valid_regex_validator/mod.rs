use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
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
) -> Result<(), ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type
        .schema
        .recursive_schema_validator_versions
        .pattern_is_valid_regex_validator
    {
        0 => Ok(v0::pattern_is_valid_regex_validator_v0(
            path, key, parent, value, result,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "pattern_is_valid_regex_validator".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
