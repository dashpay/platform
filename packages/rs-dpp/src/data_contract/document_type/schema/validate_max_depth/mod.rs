use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

mod v0;

pub fn validate_max_depth(
    value: &Value,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type
        .schema
        .validate_max_depth
    {
        0 => Ok(v0::validate_max_depth_v0(value)),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "validate_max_depth".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
