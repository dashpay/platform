use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

mod v0;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MaxDepthValidationResult {
    pub depth: u16,
    pub size: u64,
}

pub fn validate_max_depth(
    value: &Value,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<MaxDepthValidationResult>, ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .validate_max_depth
    {
        0 => Ok(v0::validate_max_depth_v0(value, platform_version)),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "validate_max_depth".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
