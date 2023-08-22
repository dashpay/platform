use crate::validation::JsonSchemaValidator;
use jsonschema::{JSONSchema, KeywordDefinition};
use serde_json::json;

impl JsonSchemaValidator {
    pub(super) fn get_schema_compilation_options_v0() -> jsonschema::CompilationOptions {
        JSONSchema::options().add_keyword(
            "byteArray",
            KeywordDefinition::Schema(json!({
                "items": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 255,
                },
            })),
        )
    }
}
