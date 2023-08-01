use crate::consensus::ConsensusError;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::{NonConsensusError, SerdeParsingError};
use serde_json::Value as JsonValue;

impl JsonSchemaValidator {
    pub(super) fn validate_v0(
        &self,
        object: &JsonValue,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        // TODO: create better error messages
        let res = self
            .schema
            .as_ref()
            .ok_or_else(|| SerdeParsingError::new("Expected schema to be initialized"))?
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
}
