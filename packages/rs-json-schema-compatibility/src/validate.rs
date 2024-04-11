use crate::change::PatchOperationPath;
use crate::error::{
    Error, InvalidJsonPatchOperationPathError, UndefinedReplaceCallbackError,
    UnsupportedSchemaKeywordError,
};
use crate::{JsonSchemaChange, KeywordRule, KEYWORD_RULES};
use json_patch::PatchOperation;
use serde_json::Value;

pub struct CompatibilityValidationResult {
    incompatible_changes: Vec<JsonSchemaChange>,
}

impl CompatibilityValidationResult {
    pub fn is_compatible(&self) -> bool {
        self.incompatible_changes.is_empty()
    }

    pub fn incompatible_changes(&self) -> &[JsonSchemaChange] {
        &self.incompatible_changes
    }

    pub fn into_changes(self) -> Vec<JsonSchemaChange> {
        self.incompatible_changes
    }
}

pub fn validate_schemas_compatibility(
    original_schema: &Value,
    new_schema: &Value,
) -> Result<CompatibilityValidationResult, Error> {
    let patch = json_patch::diff(original_schema, new_schema);
    let mut incompatible_changes: Vec<JsonSchemaChange> = Vec::new();

    for operation in patch.0.into_iter() {
        let path = operation.path();

        let Some((keyword, rule)) = find_keyword_rule(path)? else {
            return Err(Error::InvalidJsonPatchOperationPath(
                InvalidJsonPatchOperationPathError {
                    path: path.to_string(),
                },
            ));
        };

        let is_compatible = match &operation {
            PatchOperation::Add(_) => rule.allow_adding,
            PatchOperation::Remove(_) => rule.allow_removing,
            PatchOperation::Replace(op) => {
                let callback = rule
                    .allow_replacing
                    .as_ref()
                    .ok_or(UndefinedReplaceCallbackError { keyword })?;

                callback(original_schema, op)?
            }
            PatchOperation::Test(_) | PatchOperation::Copy(_) | PatchOperation::Move(_) => {
                unreachable!(
                    "json_patch diff doesn't return decorative operations test, copy, move"
                )
            }
        };

        if !is_compatible {
            incompatible_changes.push(operation.try_into()?);
        }
    }

    Ok(CompatibilityValidationResult {
        incompatible_changes,
    })
}

fn find_keyword_rule(path: &str) -> Result<Option<(String, &KeywordRule)>, Error> {
    let mut path_segments = path.split('/');

    // Remove the first empty segment
    path_segments.next();

    let mut latest_keyword_rule: Option<(String, &KeywordRule)> = None;
    let mut levels_to_subschema: Option<usize> = None;
    for segment in path_segments {
        // Switch to inner rule if it's present if we have more
        // segments after the keyword
        if let Some((keyword, rule)) = &latest_keyword_rule {
            if let Some(inner_rule) = &rule.inner {
                latest_keyword_rule = Some((keyword.clone(), inner_rule));
            }
        }

        // Skip levels to a next keyword if we expect an inner subschema
        if let Some(levels) = levels_to_subschema {
            if levels - 1 > 0 {
                levels_to_subschema = levels.checked_sub(1);

                continue;
            }
        } else if latest_keyword_rule.is_some() {
            // Continue if we don't expect an inner subschema
            continue;
        }

        let rule = KEYWORD_RULES
            .get(segment)
            .ok_or(UnsupportedSchemaKeywordError {
                keyword: segment.to_string(),
                path: path.to_string(),
            })?;

        levels_to_subschema = rule.levels_to_subschema;

        latest_keyword_rule = Some((segment.to_string(), rule));
    }

    Ok(latest_keyword_rule)
}

#[cfg(any(test, feature = "examples"))]
mod tests {
    use super::*;

    #[test]
    fn test_find_a_rule_for_properties() {
        let result =
            find_keyword_rule("/properties/prop1").expect("should find keyword without failure");

        assert_eq!(
            result,
            Some((
                "properties".to_string(),
                KEYWORD_RULES
                    .get("properties")
                    .unwrap()
                    .inner
                    .as_deref()
                    .unwrap()
            ))
        );

        let result = find_keyword_rule("/properties/prop1/properties/type")
            .expect("failed to find a keyword rule");

        assert_eq!(
            result,
            Some((
                "properties".to_string(),
                KEYWORD_RULES
                    .get("properties")
                    .unwrap()
                    .inner
                    .as_deref()
                    .unwrap()
            ))
        );
    }
}
