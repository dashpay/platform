use std::collections::BTreeMap;

use jsonschema::{JSONSchema, KeywordDefinition};
use serde_json::{json, Value};

use crate::consensus::ConsensusError;
use crate::util::json_value::JsonValueExt;
use crate::validation::ValidationResult;
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};

use super::meta_validators;

pub struct JsonSchemaValidator {
    raw_schema_json: Value,
    schema: Option<JSONSchema>,
}

impl JsonSchemaValidator {
    pub fn new(schema_json: Value) -> Result<Self, DashPlatformProtocolInitError> {
        let mut json_schema_validator = Self {
            raw_schema_json: schema_json,
            schema: None,
        };

        let compilation_options = Self::get_schema_compilation_options();
        let json_schema = compilation_options.compile(&json_schema_validator.raw_schema_json)?;
        json_schema_validator.schema = Some(json_schema);

        Ok(json_schema_validator)
    }

    pub fn new_with_definitions(
        mut schema_json: Value,
        definitions: &BTreeMap<String, Value>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let _ = schema_json.insert(String::from("$defs"), json!(definitions));

        let mut json_schema_validator = Self {
            raw_schema_json: schema_json,
            schema: None,
        };

        let compilation_options = Self::get_schema_compilation_options();
        let json_schema = compilation_options.compile(&json_schema_validator.raw_schema_json)?;
        json_schema_validator.schema = Some(json_schema);

        Ok(json_schema_validator)
    }

    pub fn validate(&self, object: &Value) -> Result<ValidationResult<()>, NonConsensusError> {
        // TODO: create better error messages
        let res = self
            .schema
            .as_ref()
            .ok_or_else(|| SerdeParsingError::new("Expected identity schema to be initialized"))?
            .validate(object);

        let mut validation_result = ValidationResult::new(None);

        match res {
            Ok(_) => Ok(validation_result),
            Err(validation_errors) => {
                let errors: Vec<ConsensusError> =
                    validation_errors.map(ConsensusError::from).collect();
                validation_result.add_errors(errors);
                Ok(validation_result)
            }
        }
    }

    /// validates schema through compilation
    pub fn validate_schema(schema: &Value) -> ValidationResult<()> {
        let mut validation_result = ValidationResult::new(None);

        let res = JSONSchema::options()
            .should_ignore_unknown_formats(false)
            .should_validate_formats(true)
            .compile(schema);
        match res {
            Ok(_) => validation_result,
            Err(validation_error) => {
                validation_result.add_error(ConsensusError::from(validation_error));
                validation_result
            }
        }
    }

    /// Uses predefined meta-schemas to validate data contract schema
    pub fn validate_data_contract_schema(
        data_contract_schema: &Value,
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        let mut validation_result = ValidationResult::new(None);
        let res = meta_validators::DATA_CONTRACT_META_SCHEMA.validate(data_contract_schema);

        match res {
            Ok(_) => Ok(validation_result),
            Err(validation_errors) => {
                let errors: Vec<ConsensusError> =
                    validation_errors.map(ConsensusError::from).collect();
                validation_result.add_errors(errors);
                Ok(validation_result)
            }
        }
    }

    fn get_schema_compilation_options() -> jsonschema::CompilationOptions {
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
