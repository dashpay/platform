use crate::data_contract::document_type::schema::IncompatibleJsonSchemaOperation;
use crate::data_contract::errors::{DataContractError, JsonSchemaError};
use crate::data_contract::JsonValue;
use crate::validation::SimpleValidationResult;
use crate::ProtocolError;
use json_schema_compatibility_validator::validate_schemas_compatibility;

pub(super) fn validate_schema_compatibility_v0(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
) -> Result<SimpleValidationResult<IncompatibleJsonSchemaOperation>, ProtocolError> {
    validate_schemas_compatibility(original_schema, new_schema)
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
