use jsonschema::{JSONSchema, KeywordDefinition};
use serde_json::{json, Value};
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};
use crate::consensus::ConsensusError;
use crate::validation::{byte_array_meta, ValidationResult};

pub struct JsonSchemaValidator {
    raw_schema_json: Value,
    schema: Option<JSONSchema>
}

impl JsonSchemaValidator {
    pub fn new(schema_json: Value) -> Result<Self, DashPlatformProtocolInitError> {
        let mut json_schema_validator = Self {
            raw_schema_json: schema_json,
            schema: None,
        };

        // BYTE_ARRAY META SCHEMA
        let schema_clone = &json_schema_validator.raw_schema_json.clone();
        let res = byte_array_meta::validate(&schema_clone);

        match res {
            Ok(_) => {}
            Err(mut errors) => {
                return Err(DashPlatformProtocolInitError::from(errors.remove(0)));
            }
        }
        // BYTE_ARRAY META SCHEMA END

        let json_schema = JSONSchema::options()
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
            .compile(&json_schema_validator.raw_schema_json)?;
        json_schema_validator.schema = Some(json_schema);

        Ok(json_schema_validator)
    }

    pub fn validate(&self, object: &Value) -> Result<ValidationResult, NonConsensusError> {
        // TODO: create better error messages
        let res = self
            .schema
            .as_ref()
            .ok_or(SerdeParsingError::new(
                "Expected identity schema to be initialized",
            ))?
            .validate(&object);

        let mut validation_result = ValidationResult::new(None);

        return match res {
            Ok(_) => { Ok(validation_result) }
            Err(validation_errors) => {
                let errors: Vec<ConsensusError> =
                    validation_errors.map(|e| ConsensusError::from(e)).collect();
                validation_result.add_errors(errors);
                Ok(validation_result)
            }
        }
    }
}
