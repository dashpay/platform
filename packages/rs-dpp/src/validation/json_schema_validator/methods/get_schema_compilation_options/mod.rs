mod v0;

use crate::validation::JsonSchemaValidator;
use crate::version::PlatformVersion;
use jsonschema::{JSONSchema, KeywordDefinition};
use serde_json::json;

impl JsonSchemaValidator {
    pub(in crate::validation::json_schema_validator) fn get_schema_compilation_options(
        platform_version: &PlatformVersion,
    ) -> jsonschema::CompilationOptions {
        JSONSchema::options()
            .should_ignore_unknown_formats(false)
            .should_validate_formats(true)
            .add_keyword(
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
