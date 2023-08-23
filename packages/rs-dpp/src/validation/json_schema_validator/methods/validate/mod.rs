use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::version::PlatformVersion;
use crate::NonConsensusError;
use serde_json::Value as JsonValue;

mod v0;

impl JsonSchemaValidator {
    pub fn validate(
        &self,
        object: &JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        match platform_version
            .dpp
            .validation
            .json_schema_validator
            .validate
        {
            0 => self.validate_v0(object),
            version => Err(NonConsensusError::UnknownVersionMismatch {
                method: "JsonSchemaValidator::validate".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
