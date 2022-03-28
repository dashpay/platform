use std::collections::HashMap;
use crate::validation::ValidationResult;
use jsonschema::JSONSchema;
use serde_json::Value as JsonValue;
use crate::errors::consensus::basic::JsonSchemaError;

// pub struct KeywordDefinition {
//     keyword: String,
//     validation: dyn Fn(JSONSchema, JSONSchema) -> Result<bool, JsonSchemaError>
// }

pub struct JsonSchemaValidator {
    //keywords: HashMap<String, KeywordDefinition>
}

impl JsonSchemaValidator {
    pub fn new() -> Self {
        Self {
            //keywords: HashMap::new(),
        }
    }

    pub fn validate(schema: JSONSchema, object: JsonValue, additional_schemas: Option<Vec<JSONSchema>>) -> ValidationResult {
        ValidationResult::new(None)
    }

    // pub fn add_keyword(&mut self, keyword_definition: Box<KeywordDefinition>) {
    //
    // }
}