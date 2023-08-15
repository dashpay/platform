use crate::ProtocolError;
use jsonschema::JSONSchema;
use platform_value::Value;
use platform_version::version::PlatformVersion;

mod v0;
pub(in crate::data_contract) fn create_validator(
    schema: &Value,
    platform_version: &PlatformVersion,
) -> Result<JSONSchema, ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type
        .schema
        .create_validator
    {
        0 => v0::create_validator_v0(schema),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "create_validator".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
