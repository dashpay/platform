mod v0;

use crate::validation::JsonSchemaValidator;
use crate::version::PlatformVersion;
use crate::DashPlatformProtocolInitError;
use serde_json::Value as JsonValue;

impl JsonSchemaValidator {
    pub fn new_with_definitions<'a>(
        schema_json: JsonValue,
        definitions: impl IntoIterator<Item = (&'a String, &'a JsonValue)>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        match platform_version
            .dpp
            .validation
            .json_schema_validator
            .new_with_definitions
        {
            0 => Self::new_with_definitions_v0(schema_json, definitions, platform_version),
            version => Err(DashPlatformProtocolInitError::UnknownVersionMismatch {
                method: "JsonSchemaValidator::new_with_definitions".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
