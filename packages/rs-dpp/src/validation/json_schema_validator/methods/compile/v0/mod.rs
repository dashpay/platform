use crate::consensus::ConsensusError;
use crate::data_contract::JsonValue;
use crate::validation::JsonSchemaValidator;
use crate::ProtocolError;
use jsonschema::{JSONSchema, KeywordDefinition};
use serde_json::json;

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
            .should_ignore_unknown_formats(false)
            .should_validate_formats(true)
            .with_draft(jsonschema::Draft::Draft202012)
            .clone() // doesn't work otherwise
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
            .compile(json_schema)
            .map_err(|error| {
                ProtocolError::ConsensusError(Box::new(ConsensusError::from(error)))
            })?;

        *validator_guard = Some(validator);

        Ok(true)
    }

    #[inline(always)]
    pub(super) fn is_compiled_v0(&self) -> bool {
        let validator_guard = self.validator.read().unwrap();

        validator_guard.is_some()
    }
}
