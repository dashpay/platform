use platform_value::Value;
use platform_version::version::PlatformVersion;
use crate::data_contract::document_type::schema::recursive_schema_validator::traversal_validator::v0::SubValidator;
use crate::ProtocolError;
use crate::validation::SimpleConsensusValidationResult;

mod v0;

pub fn traversal_validator(
    raw_data_contract: &Value,
    validators: &[SubValidator],
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .recursive_schema_validator_versions
        .traversal_validator
    {
        0 => v0::traversal_validator_v0(raw_data_contract, validators, platform_version),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "DocumentMethodV0::hash".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
