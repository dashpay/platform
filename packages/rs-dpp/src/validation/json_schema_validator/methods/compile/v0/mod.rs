use crate::consensus::ConsensusError;
use crate::data_contract::JsonValue;
use crate::validation::byte_array_keyword::ByteArrayKeyword;
use crate::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};
use crate::ProtocolError;
use jsonschema::{JSONSchema, RegexEngine, RegexOptions};

impl JsonSchemaValidator {
    #[inline(always)]
    pub(super) fn compile_v0(&self, json_schema: &JsonValue) -> Result<bool, ProtocolError> {
        if self.is_compiled_v0() {
            return Ok(false);
        }

        let mut validator_guard = self.validator.write().unwrap();

        // Check again to ensure no other thread has modified it after dropping the read lock
        if validator_guard.is_some() {
            return Ok(false);
        }

        let validator = JSONSchema::options()
            .with_meta_schemas()
            .with_patterns_regex_engine(RegexEngine::Regex(RegexOptions {
                size_limit: Some(5 * (1 << 20)),
                ..Default::default()
            }))
            .should_ignore_unknown_formats(false)
            .should_validate_formats(true)
            .with_draft(jsonschema::Draft::Draft202012)
            .with_keyword("byteArray", |_, _, _| Ok(Box::new(ByteArrayKeyword)))
            .compile(json_schema)
            .map_err(|error| {
                ProtocolError::ConsensusError(Box::new(ConsensusError::from(error)))
            })?;

        *validator_guard = Some(validator);

        Ok(true)
    }

    #[inline(always)]
    pub(super) fn compile_and_validate_v0(
        &self,
        json_schema: &JsonValue,
        json_value: &JsonValue,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut validator_guard = self.validator.write().unwrap();

        // Check again to ensure no other thread has modified it after dropping the read lock
        if let Some(validator) = validator_guard.as_ref() {
            match validator.validate(json_value) {
                Ok(_) => Ok(SimpleConsensusValidationResult::new()),
                Err(validation_errors) => {
                    let errors: Vec<ConsensusError> =
                        validation_errors.map(ConsensusError::from).collect();

                    Ok(SimpleConsensusValidationResult::new_with_errors(errors))
                }
            }
        } else {
            let validator = JSONSchema::options()
                .with_meta_schemas()
                .with_patterns_regex_engine(RegexEngine::Regex(Default::default()))
                .should_ignore_unknown_formats(false)
                .should_validate_formats(true)
                .with_draft(jsonschema::Draft::Draft202012)
                .with_keyword("byteArray", |_, _, _| Ok(Box::new(ByteArrayKeyword)))
                .compile(json_schema)
                .map_err(|error| {
                    // Todo: not sure this is a consensus error
                    ProtocolError::ConsensusError(Box::new(ConsensusError::from(error)))
                })?;

            let validation_result = if let Err(validation_errors) = validator.validate(json_value) {
                let errors: Vec<ConsensusError> =
                    validation_errors.map(ConsensusError::from).collect();

                Ok(SimpleConsensusValidationResult::new_with_errors(errors))
            } else {
                Ok(SimpleConsensusValidationResult::new())
            };

            *validator_guard = Some(validator);

            validation_result
        }
    }

    #[inline(always)]
    pub(super) fn is_compiled_v0(&self) -> bool {
        let validator_guard = self.validator.read().unwrap();

        validator_guard.is_some()
    }
}
