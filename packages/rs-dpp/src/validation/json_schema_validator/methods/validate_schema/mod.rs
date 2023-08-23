mod v0;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

impl JsonSchemaValidator {
    /// validates schema through compilation
    pub fn validate_schema(
        schema: &JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .json_schema_validator
            .validate_schema
        {
            0 => Ok(Self::validate_schema_v0(schema)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "JsonSchemaValidator::validate_schema".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
