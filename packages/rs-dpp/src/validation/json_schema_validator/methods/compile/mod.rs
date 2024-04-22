use crate::data_contract::JsonValue;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

impl JsonSchemaValidator {
    pub fn compile(
        &self,
        json_schema: &JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .json_schema_validator
            .compile
        {
            0 => self.compile_v0(json_schema),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "JsonSchemaLazyValidator.compile".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn compile_and_validate(
        &self,
        json_schema: &JsonValue,
        json_value: &JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .json_schema_validator
            .compile_and_validate
        {
            0 => self.compile_and_validate_v0(json_schema, json_value),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "JsonSchemaLazyValidator.compile_and_validate".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn is_compiled(&self, platform_version: &PlatformVersion) -> Result<bool, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .json_schema_validator
            .compile
        {
            0 => Ok(self.is_compiled_v0()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "JsonSchemaLazyValidator.is_compiled".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
