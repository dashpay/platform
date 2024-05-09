use crate::consensus::basic::json_schema_compilation_error::JsonSchemaCompilationError;
use crate::consensus::basic::value_error::ValueError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Value;

#[inline(always)]
pub(super) fn byte_array_has_no_items_as_parent_validator_v0(
    path: &str,
    key: &str,
    parent: &Value,
    value: &Value,
    result: &mut SimpleConsensusValidationResult,
) {
    if key == "byteArray"
        && value.is_bool()
        && (unwrap_error_to_result(
            parent.get("items").map_err(|e| {
                ConsensusError::BasicError(BasicError::ValueError(ValueError::new(e)))
            }),
            result,
        )
        .is_some()
            || unwrap_error_to_result(
                parent.get("prefixItems").map_err(|e| {
                    ConsensusError::BasicError(BasicError::ValueError(ValueError::new(e)))
                }),
                result,
            )
            .is_some())
    {
        let compilation_error = format!(
            "invalid path: '{}': byteArray cannot be used with 'items' or 'prefixItems",
            path
        );
        result.add_error(BasicError::JsonSchemaCompilationError(
            JsonSchemaCompilationError::new(compilation_error),
        ));
    }
}

fn unwrap_error_to_result<'a>(
    v: Result<Option<&'a Value>, ConsensusError>,
    result: &mut SimpleConsensusValidationResult,
) -> Option<&'a Value> {
    match v {
        Ok(v) => v,
        Err(e) => {
            result.add_error(e);
            None
        }
    }
}
