use dpp::state_transition::StateTransitionFieldTypes;
use dpp::validation::{JsonSchemaValidator, SimpleConsensusValidationResult};

pub(crate) fn validate_schema_v0(
    json_schema_validator: &JsonSchemaValidator,
    state_transition: &impl StateTransitionFieldTypes,
) -> SimpleConsensusValidationResult {
    json_schema_validator
        .validate(
            &(state_transition
                .to_cleaned_object(false)
                .expect("we don't hold unserializable structs")
                .try_into_validating_json()
                .expect("TODO")),
        )
        .expect("TODO: how jsonschema validation will ever fail?")
}
