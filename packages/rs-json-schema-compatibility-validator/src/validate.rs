use crate::change::PatchOperationPath;
use crate::error::{
    Error, InvalidJsonPatchOperationPathError, UndefinedReplacementAllowedCallbackError,
    UnsupportedSchemaKeywordError,
};
use crate::{CompatibilityRules, JsonSchemaChange, KEYWORD_COMPATIBILITY_RULES};
use json_patch::PatchOperation;
use serde_json::Value;

/// The result of JSON Schema compatibility validation between two schemas.
/// Returned by [validate_schemas_compatibility] function.
pub struct CompatibilityValidationResult {
    incompatible_changes: Vec<JsonSchemaChange>,
}

impl CompatibilityValidationResult {
    /// Returns `true` if the schemas are compatible.
    pub fn is_compatible(&self) -> bool {
        self.incompatible_changes.is_empty()
    }

    /// Returns a list of incompatible changes between the schemas.
    pub fn incompatible_changes(&self) -> &[JsonSchemaChange] {
        &self.incompatible_changes
    }

    /// Consumes the result and returns a list of incompatible changes between the schemas.
    pub fn into_changes(self) -> Vec<JsonSchemaChange> {
        self.incompatible_changes
    }
}

/// Validates the backward compatibility of two JSON schemas and returns
/// the [CompatibilityValidationResult]. If two schemas are compatible,
/// it means that data that valid for the original schema is also valid
/// for the new schema.
///
/// ```
/// use serde_json::json;
/// use json_schema_compatibility_validator::validate_schemas_compatibility;
///
/// let original_schema = json!({
///     "type": "object",
///     "properties": {
///         "name": { "type": "string" },
///         "age": { "type": "integer" }
///     },
///     "required": ["name"]
/// });
///
/// let new_schema = json!({
///     "type": "object",
///     "properties": {
///         "name": { "type": "string" },
///         "age": { "type": "integer" },
///         "email": { "type": "string" }
///     },
///     "required": ["name"]
/// });
///
/// let result = validate_schemas_compatibility(&original_schema, &new_schema)
///  .expect("compatibility validation failed");
///
/// assert!(result.is_compatible());
/// ```
///
pub fn validate_schemas_compatibility(
    original_schema: &Value,
    new_schema: &Value,
) -> Result<CompatibilityValidationResult, Error> {
    let patch = json_patch::diff(original_schema, new_schema);

    let mut incompatible_changes: Vec<JsonSchemaChange> = Vec::new();

    for operation in patch.0.into_iter() {
        let path = operation.path();

        let Some(rules) = find_compatibility_rules(path)? else {
            return Err(Error::InvalidJsonPatchOperationPath(
                InvalidJsonPatchOperationPathError {
                    path: path.to_string(),
                },
            ));
        };

        if !is_compatible_operation(original_schema, &operation, rules)? {
            incompatible_changes.push(operation.try_into()?);
        }
    }

    Ok(CompatibilityValidationResult {
        incompatible_changes,
    })
}

/// Returns `true` if the operation is compatible with the schema
/// according to provided compatibility rules, otherwise `false`.
fn is_compatible_operation(
    original_schema: &Value,
    operation: &PatchOperation,
    rules: &CompatibilityRules,
) -> Result<bool, Error> {
    match &operation {
        PatchOperation::Add(_) => Ok(rules.allow_addition),
        PatchOperation::Remove(_) => Ok(rules.allow_removal),
        PatchOperation::Replace(op) => {
            let callback = rules.allow_replacement_callback.as_ref().ok_or_else(|| {
                UndefinedReplacementAllowedCallbackError {
                    path: op.path.clone(),
                    rules: rules.clone(),
                }
            })?;

            callback(original_schema, op)
        }
        PatchOperation::Test(_) | PatchOperation::Copy(_) | PatchOperation::Move(_) => {
            unreachable!("json_patch diff doesn't return decorative operations test, copy, move")
        }
    }
}

/// Travers through the JSON Pointer path and find corresponding compatibility rules
fn find_compatibility_rules(path: &str) -> Result<Option<&CompatibilityRules>, Error> {
    let mut path_segments = path.split('/');

    // Remove the first empty segment
    path_segments.next();

    let mut latest_keyword_compatibility_rules: Option<&CompatibilityRules> = None;
    let mut levels_to_subschema: Option<usize> = None;
    for segment in path_segments {
        // On the second iteration we look at the inner levels under the keyword.
        // Switch to inner structure rules if they are present
        if let Some(rule) = latest_keyword_compatibility_rules {
            if let Some(inner_rule) = &rule.inner {
                latest_keyword_compatibility_rules = Some(inner_rule);
            }
        }

        // Skip some levels to a next keyword if we expect an inner subschema
        if let Some(levels) = levels_to_subschema {
            if levels - 1 > 0 {
                levels_to_subschema = levels.checked_sub(1);

                continue;
            }
        } else if latest_keyword_compatibility_rules.is_some() {
            // Continue if we don't expect an inner subschema
            continue;
        }

        // The first segment is always a keyword
        let rules = KEYWORD_COMPATIBILITY_RULES.get(segment).ok_or_else(|| {
            UnsupportedSchemaKeywordError {
                keyword: segment.to_string(),
                path: path.to_string(),
            }
        })?;

        levels_to_subschema = rules.subschema_levels_depth;

        latest_keyword_compatibility_rules = Some(rules);
    }

    Ok(latest_keyword_compatibility_rules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_compatibility_rules() {
        let result = find_compatibility_rules("/properties/prop1")
            .expect("should find keyword without failure");

        assert_eq!(
            result,
            Some(
                KEYWORD_COMPATIBILITY_RULES
                    .get("properties")
                    .unwrap()
                    .inner
                    .as_deref()
                    .unwrap()
            )
        );

        let result = find_compatibility_rules("/properties/prop1/properties/type")
            .expect("failed to find a keyword rule");

        assert_eq!(
            result,
            Some(
                KEYWORD_COMPATIBILITY_RULES
                    .get("properties")
                    .unwrap()
                    .inner
                    .as_deref()
                    .unwrap()
            )
        );
    }
}
