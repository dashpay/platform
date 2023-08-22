mod v0;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

impl JsonSchemaValidator {
    /// validates schema through compilation
    pub fn validate_data_contract_schema(
        data_contract_schema: &JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .json_schema_validator
            .validate_data_contract_schema
        {
            0 => Ok(Self::validate_data_contract_schema_v0(data_contract_schema)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "JsonSchemaValidator::validate_data_contract_schema".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
