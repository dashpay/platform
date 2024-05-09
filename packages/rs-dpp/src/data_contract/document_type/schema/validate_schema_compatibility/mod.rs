use crate::data_contract::JsonValue;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

use crate::validation::SimpleValidationResult;

#[derive(Debug, Clone)]
pub struct IncompatibleJsonSchemaOperation {
    pub name: String,
    pub path: String,
}

pub fn validate_schema_compatibility(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
    platform_version: &PlatformVersion,
) -> Result<SimpleValidationResult<IncompatibleJsonSchemaOperation>, ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .validate_schema_compatibility
    {
        0 => v0::validate_schema_compatibility_v0(original_schema, new_schema),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "validate_schema_compatibility".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
