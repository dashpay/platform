use crate::consensus::ConsensusError;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use jsonschema::JSONSchema;
use serde_json::Value as JsonValue;

impl JsonSchemaValidator {
    /// validates schema through compilation
    pub fn validate_schema_v0(schema: &JsonValue) -> SimpleConsensusValidationResult {
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
}
