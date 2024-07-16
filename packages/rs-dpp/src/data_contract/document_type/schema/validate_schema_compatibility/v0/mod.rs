use crate::data_contract::document_type::schema::IncompatibleJsonSchemaOperation;
use crate::data_contract::errors::{DataContractError, JsonSchemaError};
use crate::data_contract::JsonValue;
use crate::validation::SimpleValidationResult;
use crate::ProtocolError;
use json_schema_compatibility_validator::{
    validate_schemas_compatibility, CompatibilityRulesCollection, Options,
    KEYWORD_COMPATIBILITY_RULES,
};
use once_cell::sync::Lazy;
use std::ops::Deref;

static OPTIONS: Lazy<Options> = Lazy::new(|| {
    let mut required_rule = KEYWORD_COMPATIBILITY_RULES
        .get("required")
        .expect("required rule must be present")
        .clone();

    required_rule.allow_removal = false;
    required_rule
        .inner
        .as_mut()
        .expect("required rule must have inner rules")
        .allow_removal = false;

    Options {
        override_rules: CompatibilityRulesCollection::from_iter([("required", required_rule)]),
    }
});

pub(super) fn validate_schema_compatibility_v0(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
) -> Result<SimpleValidationResult<IncompatibleJsonSchemaOperation>, ProtocolError> {
    validate_schemas_compatibility(original_schema, new_schema, OPTIONS.deref())
        .map(|result| {
            let errors = result
                .into_changes()
                .into_iter()
                .map(|change| IncompatibleJsonSchemaOperation {
                    name: change.name().to_string(),
                    path: change.path().to_string(),
                })
                .collect::<Vec<_>>();

            SimpleValidationResult::new_with_errors(errors)
        })
        .map_err(|error| {
            ProtocolError::DataContractError(DataContractError::JsonSchema(
                JsonSchemaError::SchemaCompatibilityValidationError(error.to_string()),
            ))
        })
}
