use crate::validation::ValidationResult;
use jsonschema::JSONSchema;
use serde_json::Value as JsonValue;

pub struct JsonSchemaValidator {

}

impl JsonSchemaValidator {
    pub fn new() -> Self {
        Self { }
    }

    pub fn validate(schema: JSONSchema, object: JsonValue, additional_schemas: Option<Vec<JSONSchema>>) -> ValidationResult {
        ValidationResult::new(None)
    }
}