mod v0;

use crate::data_contract::JsonValue;
use crate::validation::JsonSchemaValidator;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use std::sync::RwLock;

impl Default for JsonSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonSchemaValidator {
    pub fn new() -> Self {
        Self {
            validator: RwLock::new(None),
        }
    }

    pub fn new_compiled(
        json_schema: &JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version.dpp.validation.json_schema_validator.new {
            0 => Self::new_compiled_v0(json_schema, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "JsonSchemaValidator::new_compiled".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
