use crate::consensus::ConsensusError;
use crate::validation::{meta_validators, JsonSchemaValidator, SimpleConsensusValidationResult};
use jsonschema::JSONSchema;
use serde_json::Value as JsonValue;

impl JsonSchemaValidator {
    /// Uses predefined meta-schemas to validate data contract schema
    pub(super) fn validate_data_contract_schema_v0(
        data_contract_schema: &JsonValue,
    ) -> SimpleConsensusValidationResult {
        let mut validation_result = SimpleConsensusValidationResult::default();
        let res = meta_validators::DOCUMENT_META_SCHEMA_V0.validate(data_contract_schema);

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
}
