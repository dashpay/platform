use crate::consensus::ConsensusError;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::ProtocolError;
use jsonschema::{JSONSchema, KeywordDefinition};
use platform_value::Value;
use serde_json::json;

pub(super) fn create_validator_v0(schema: &Value) -> Result<JSONSchema, ProtocolError> {
    let json_schema = schema
        .try_to_validating_json()
        .map_err(ProtocolError::ValueError)?;

    JSONSchema::options()
        .should_ignore_unknown_formats(false)
        .should_validate_formats(true)
        .with_draft(jsonschema::Draft::Draft202012)
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
        .compile(&json_schema)
        .map_err(|error| ProtocolError::ConsensusError(Box::new(ConsensusError::from(error))))
}
