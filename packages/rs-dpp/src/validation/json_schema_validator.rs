use super::meta_validators;
use jsonschema::{JSONSchema, KeywordDefinition};
use serde_json::{json, Value};

use crate::consensus::ConsensusError;
use crate::validation::ValidationResult;
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};

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

        // BYTE_ARRAY META SCHEMA
        // let schema_clone = &json_schema_validator.raw_schema_json.clone();
        // let res = byte_array_meta::validate(&schema_clone);
        //
        // match res {
        //     Ok(_) => {}
        //     Err(mut errors) => {
        //         return Err(DashPlatformProtocolInitError::from(errors.remove(0)));
        //     }
        // }
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
    pub fn validate_schema(schema: &Value) -> ValidationResult {
        let mut validation_result = ValidationResult::new(None);

        // enable the format validation
        // let res = meta_validators::DRAFT_202012_META_SCHEMA.validate(schema);
        // match res {
        //     Ok(_) => {}
        //     Err(validation_errors) => {
        //         let errors: Vec<ConsensusError> =
        //             validation_errors.map(ConsensusError::from).collect();
        //         validation_result.add_errors(errors);
        //         return validation_result;
        //     }
        // }

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
    ) -> Result<ValidationResult, NonConsensusError> {
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
}
