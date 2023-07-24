use crate::validation::JsonSchemaValidator;
use crate::DashPlatformProtocolInitError;
use serde_json::Value as JsonValue;

impl JsonSchemaValidator {
    pub(super) fn new_v0(schema_json: JsonValue) -> Result<Self, DashPlatformProtocolInitError> {
        let mut json_schema_validator = Self {
            raw_schema_json: schema_json,
            schema: None,
        };

        let mut compilation_options = Self::get_schema_compilation_options();
        let json_schema = compilation_options
            .with_draft(jsonschema::Draft::Draft202012)
            .compile(&json_schema_validator.raw_schema_json)?;
        json_schema_validator.schema = Some(json_schema);

        Ok(json_schema_validator)
    }
}
