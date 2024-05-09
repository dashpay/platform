use crate::consensus::ConsensusError;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::ProtocolError;

use serde_json::Value as JsonValue;

impl JsonSchemaValidator {
    #[inline(always)]
    pub(super) fn validate_v0(
        &self,
        instance: &JsonValue,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let validator_guard = self.validator.read().unwrap();

        let Some(validator) = validator_guard.as_ref() else {
            return Err(ProtocolError::Generic(
                "validator is not compiled".to_string(),
            ));
        };

        // TODO: create better error messages
        let result = validator.validate(instance);

        match result {
            Ok(_) => Ok(SimpleConsensusValidationResult::default()),
            Err(validation_errors) => {
                let errors: Vec<ConsensusError> =
                    validation_errors.map(ConsensusError::from).collect();

                Ok(SimpleConsensusValidationResult::new_with_errors(errors))
            }
        }
    }
}
