use crate::validation::SimpleConsensusValidationResult;
use platform_value::Value;
use platform_version::version::PlatformVersion;

mod v0;

pub fn validate_max_depth(
    value: &Value,
    platform_version: &PlatformVersion,
) -> SimpleConsensusValidationResult {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .validation_versions
        .validate_max_depth
    {
        0 => v0::validate_max_depth_v0(value),
        version => unimplemented!("validate_data_contract_max_depth_v{}", version),
    }
}
