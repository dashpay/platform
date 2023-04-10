use dpp::{
    state_transition::StateTransitionConvert,
    validation::{JsonSchemaValidator, SimpleConsensusValidationResult},
    version::ProtocolVersionValidator,
};
use drive::grovedb::Transaction;

use crate::error::Error;

pub(super) fn validate_schema(
    json_schema_validator: &JsonSchemaValidator,
    state_transition: &impl StateTransitionConvert,
) -> SimpleConsensusValidationResult {
    json_schema_validator
        .validate(
            &(state_transition
                .to_object(false)
                .expect("we don't hold unserializable structs")
                .try_into_validating_json()
                .expect("TODO")),
        )
        .expect("TODO: how jsonschema validation will ever fail?")
}

pub(super) fn validate_protocol_version(protocol_version: u32) -> SimpleConsensusValidationResult {
    let protocol_version_validator = ProtocolVersionValidator::default();
    protocol_version_validator
        .validate(protocol_version)
        .expect("TODO: again, how this will ever fail, why do we even need a validator trait")
}
