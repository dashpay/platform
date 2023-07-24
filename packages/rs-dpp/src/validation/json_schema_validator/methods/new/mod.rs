mod v0;

use crate::validation::JsonSchemaValidator;
use crate::version::PlatformVersion;
use crate::DashPlatformProtocolInitError;
use serde_json::Value as JsonValue;

impl JsonSchemaValidator {
    pub fn new(
        schema_json: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        match platform_version.dpp.validation.json_schema_validator.new {
            0 => Self::new_v0(schema_json),
            version => Err(DashPlatformProtocolInitError::UnknownVersionMismatch {
                method: "JsonSchemaValidator::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
