mod change;
mod errors;
mod keywords;

use std::collections::HashSet;
/// The Schema compatibility validator is a port of a JavaScript version
/// https://bitbucket.org/atlassian/json-schema-diff-validator/src/master/
///
/// The functionality has been ported 'as is' without any logic improvements and optimizations
use std::convert::TryFrom;

use crate::change::JsonSchemaChange;
use crate::errors::{Error, InvalidJsonPointerPathError, JsonPointerPathNotFoundError};
use crate::keywords::KEYWORD_RULES;
use json_patch::PatchOperation;
pub use json_patch::{AddOperation, RemoveOperation, ReplaceOperation};
use jsonptr::{Pointer, Resolve};
use serde_json::Value as JsonValue;

struct RemovedItem {
    name: String,
    change: JsonSchemaChange,
}

pub struct CompatibilityValidationResult {
    incompatible_changes: Vec<JsonSchemaChange>,
}

impl CompatibilityValidationResult {
    fn is_compatible(&self) -> bool {
        self.incompatible_changes.is_empty()
    }

    fn incompatible_changes(&self) -> &[JsonSchemaChange] {
        &self.incompatible_changes
    }

    fn into_changes(self) -> Vec<JsonSchemaChange> {
        self.incompatible_changes
    }
}

#[derive(Default, Debug, Clone)]
pub struct ValidationOptions {
    pub allow_reorder: bool,
}

pub fn validate_schemas_compatibility(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
) -> Result<CompatibilityValidationResult, Error> {
    validate_schema_compatibility_with_options(
        original_schema,
        new_schema,
        ValidationOptions::default(),
    )
}

pub fn validate_schema_compatibility_with_options(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
    opts: ValidationOptions,
) -> Result<CompatibilityValidationResult, Error> {
    let patch = json_patch::diff(original_schema, new_schema);
    let mut incompatible_changes: Vec<JsonSchemaChange> = Vec::new();
    let mut removed: Vec<RemovedItem> = vec![];
    let mut inserted: HashSet<String> = HashSet::new();

    for operation in patch.0.into_iter() {
        match operation {
            PatchOperation::Remove(ref op) => {
                let is_compatible = is_operation_remove_compatible(&op.path)?;

                if !is_compatible {
                    incompatible_changes.push(operation.try_into()?);
                }
            }

            PatchOperation::Replace(ref op) => {
                let is_compatible =
                    is_operation_replace_compatible(&op.path, original_schema, new_schema)?;

                if !is_compatible {
                    incompatible_changes.push(operation.try_into()?);
                }

                // if !opts.allow_reorder {
                //     incompatible_changes.push(operation.try_into()?);
                // } else {
                //     inserted.insert(op_replace.value.to_string());
                //     removed.push(RemovedItem {
                //         name: old_value.to_string(),
                //         change: operation.try_into()?,
                //     });
                // }
            }
            PatchOperation::Add(ref op) => {
                let is_compatible = is_operation_add_compatible(&op.path)?;

                if !is_compatible {
                    incompatible_changes.push(operation.try_into()?);
                }
                // let is_new_any_of_item = is_anyof_path(&op_add.path);
                // let is_new_enum_value = is_enum_path(&op_add.path);
                // let path_two_last_levels = get_second_last_sub_path(&op_add.path)
                //     // TODO: No panic
                //     .unwrap_or_else(|| {
                //         panic!("the second subpath doesn't exist in '{}'", op_add.path)
                //     });
                //
                // if path_two_last_levels == property_names::REQUIRED {
                //     incompatible_changes.push(operation.try_into()?);
                //     continue;
                // }
                // if [property_names::PROPERTIES, property_names::DEFINITIONS]
                //     .contains(&path_two_last_levels)
                // {
                //     continue;
                // }
                //
                // if is_new_any_of_item && opts.allow_reorder {
                //     // TODO: Deal with this
                //     inserted.insert(
                //         op_add
                //             .value
                //             .get(property_names::REF)
                //             .unwrap_or_else(|| &op_add.value)
                //             // .ok_or(|| {
                //             //     format!("the property '{}' doesn't exist", property_names::REF)
                //             // })?
                //             .to_string(),
                //     );
                // } else if (is_new_any_of_item && opts.allow_new_one_of)
                //     || (is_new_enum_value && opts.allow_new_enum_value)
                // {
                //     continue;
                // } else {
                //     incompatible_changes.push(operation.try_into()?)
                // }
            }
            PatchOperation::Test(_) | PatchOperation::Copy(_) | PatchOperation::Move(_) => {
                unreachable!(
                    "json_patch diff doesn't return decorative operations test, copy, move"
                )
            }
        }
    }

    // if opts.allow_reorder {
    //     // When reordering is allowed, we want ot make sure that any item that
    //     // was replaces is also inserted somewhere else
    //     let filtered_removed = removed.into_iter().filter_map(|node| {
    //         if inserted.contains(&node.name) {
    //             Some(node.change)
    //         } else {
    //             None
    //         }
    //     });
    //
    //     incompatible_changes.extend(filtered_removed);
    // }

    Ok(CompatibilityValidationResult {
        incompatible_changes,
    })
}

fn is_operation_remove_compatible(path: &str) -> Result<bool, Error> {
    let mut path_segments = path.split('/');

    let Some(last_segment) = path_segments.next_back() else {
        unreachable!("impossible to remove empty path");
    };

    if let Some(rule) = KEYWORD_RULES.get(last_segment) {
        return Ok(rule.allow_removing);
    };

    // Then it probably removal of internal element of a keyword
    let Some(second_last_segment) = path_segments.next_back() else {
        unreachable!("unsupported keywords");
    };

    if let Some(rule) = KEYWORD_RULES.get(second_last_segment) {
        let Some(inner_rule) = &rule.inner else {
            unreachable!("{second_last_segment} must have the inner rule");
        };

        return Ok(inner_rule.allow_removing);
    };

    unreachable!("unsupported keyword '{}'", path);
}

fn is_operation_add_compatible(path: &str) -> Result<bool, Error> {
    let mut path_segments = path.split('/');

    let Some(last_segment) = path_segments.next_back() else {
        unreachable!("impossible to remove empty path");
    };

    if let Some(rule) = KEYWORD_RULES.get(last_segment) {
        return Ok(rule.allow_adding);
    };

    // Then it probably removal of internal element of a keyword
    let Some(second_last_segment) = path_segments.next_back() else {
        unreachable!("unsupported keywords");
    };

    if let Some(rule) = KEYWORD_RULES.get(second_last_segment) {
        let Some(inner_rule) = &rule.inner else {
            unreachable!("{second_last_segment} must have the inner rule");
        };

        return Ok(inner_rule.allow_adding);
    };

    unreachable!("unsupported keywords");
}

fn is_operation_replace_compatible(
    path: &str,
    previous_schema: &JsonValue,
    new_schema: &JsonValue,
) -> Result<bool, Error> {
    let mut path_segments = path.split('/');

    let Some(last_segment) = path_segments.next_back() else {
        unreachable!("impossible to remove empty path");
    };

    let pointer = Pointer::try_from(path).map_err(|error| InvalidJsonPointerPathError {
        path: path.to_string(),
        error,
    })?;

    let previous_value =
        previous_schema
            .resolve(&pointer)
            .map_err(|error| JsonPointerPathNotFoundError {
                path: path.to_string(),
                error,
            })?;

    let new_value = new_schema
        .resolve(&pointer)
        .map_err(|error| JsonPointerPathNotFoundError {
            path: path.to_string(),
            error,
        })?;

    if let Some(rule) = KEYWORD_RULES.get(last_segment) {
        let callback = rule.allow_replacing.as_ref().expect("must have a callback");

        return Ok(callback(previous_value, new_value));
    };

    // Then it probably removal of internal element of a keyword
    let Some(second_last_segment) = path_segments.next_back() else {
        unreachable!("unsupported keywords");
    };

    if let Some(rule) = KEYWORD_RULES.get(second_last_segment) {
        let Some(inner_rule) = &rule.inner else {
            unreachable!("{second_last_segment} must have the inner rule");
        };

        let callback = inner_rule
            .allow_replacing
            .as_ref()
            .unwrap_or_else(|| panic!("{second_last_segment} must have the replace callback"));

        return Ok(callback(previous_value, new_value));
    };

    unreachable!("unsupported keywords");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keywords::{KeywordRule, KeywordRuleExample};
    use rstest::rstest;
    use serde_json::json;
    use test_case::{test_case, test_matrix};

    // TODO: validate
    //  - Add more prefixItems and increase array size?
    //  - Modify subschema inside `items`, `enum`, `dependentSchemas`, `prefixItems`
    //  - we should be able to reorder fields
    //  - what about $ref and $defs?

    #[test]
    fn test_schema_keyword_rules() {
        for (keyword, rule) in KEYWORD_RULES.iter() {
            println!("Testing `{}` keyword", keyword);

            assert_examples(keyword, &rule.examples);

            if let Some(inner_rule) = rule.inner.as_deref() {
                assert_examples(keyword, &inner_rule.examples);
            }
        }
    }

    fn assert_examples(keyword: &str, examples: &[KeywordRuleExample]) {
        for example in examples {
            let result =
                validate_schemas_compatibility(&example.original_schema, &example.new_schema)
                    .expect("should not fail");

            if let Some(change) = &example.incompatible_change {
                let expected_change = vec![change.clone()];

                assert_eq!(
                    result.incompatible_changes(),
                    &expected_change,
                    r"assertion failed: expected incompatible change of '{keyword}'

From: {:?}
To: {:?}",
                    &example.original_schema,
                    &example.new_schema
                );
            } else {
                assert!(
                    result.is_compatible(),
                    r"assertion failed: '{keyword}' modification is not compatible: {:?}
From: {:?}
To: {:?}",
                    result.incompatible_changes(),
                    &example.original_schema,
                    &example.new_schema
                );
            }
        }
    }
}
