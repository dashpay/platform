use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
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
) -> Result<(), ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type
        .schema
        .recursive_schema_validator_versions
        .byte_array_has_no_items_as_parent_validator
    {
        0 => {
            v0::byte_array_has_no_items_as_parent_validator_v0(path, key, parent, value, result);
            Ok(())
        }
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "byte_array_has_no_items_as_parent_validator".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
