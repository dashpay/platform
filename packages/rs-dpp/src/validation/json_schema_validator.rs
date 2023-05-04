use std::collections::HashMap;

use anyhow::Context;
use jsonschema::{JSONSchema, KeywordDefinition};
use serde_json::{json, Value as JsonValue};

use crate::consensus::ConsensusError;
use crate::util::json_value::JsonValueExt;
use crate::validation::{DataValidator, SimpleConsensusValidationResult};
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};

use super::meta_validators;

pub struct JsonSchemaValidator {
    raw_schema_json: JsonValue,
    schema: Option<JSONSchema>,
}

impl DataValidator for JsonSchemaValidator {
    type Item = JsonValue;
    fn validate(
        &self,
        data: &Self::Item,
    ) -> Result<super::SimpleConsensusValidationResult, crate::ProtocolError> {
        let result = self
            .validate(data)
            .context("error during validating json schema")?;
        Ok(result)
    }
}

impl JsonSchemaValidator {
    pub fn new(schema_json: JsonValue) -> Result<Self, DashPlatformProtocolInitError> {
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

    /// creates a new json schema validator from the json schema and allows to add the definitions
    pub fn new_with_definitions<'a>(
        mut schema_json: JsonValue,
        definitions: impl IntoIterator<Item = (&'a String, &'a JsonValue)>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let defs: HashMap<&str, &'a JsonValue> = definitions
            .into_iter()
            .map(|(k, v)| (k.as_ref(), v))
            .collect();
        let _ = schema_json.insert(String::from("$defs"), json!(defs));

        let mut json_schema_validator = Self {
            raw_schema_json: schema_json,
            schema: None,
        };

        let compilation_options = Self::get_schema_compilation_options();
        let json_schema = compilation_options.compile(&json_schema_validator.raw_schema_json)?;
        json_schema_validator.schema = Some(json_schema);

        Ok(json_schema_validator)
    }

    pub fn validate(
        &self,
        object: &JsonValue,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        // TODO: create better error messages
        let res = self
            .schema
            .as_ref()
            .ok_or_else(|| SerdeParsingError::new("Expected identity schema to be initialized"))?
            .validate(object);

        let mut validation_result = SimpleConsensusValidationResult::default();

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
    pub fn validate_schema(schema: &JsonValue) -> SimpleConsensusValidationResult {
        let mut validation_result = SimpleConsensusValidationResult::default();

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
        data_contract_schema: &JsonValue,
    ) -> SimpleConsensusValidationResult {
        let mut validation_result = SimpleConsensusValidationResult::default();
        let res = meta_validators::DATA_CONTRACT_META_SCHEMA.validate(data_contract_schema);

        match res {
            Ok(_) => validation_result,
            Err(validation_errors) => {
                let errors: Vec<ConsensusError> =
                    validation_errors.map(ConsensusError::from).collect();

                validation_result.add_errors(errors);

                validation_result
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
