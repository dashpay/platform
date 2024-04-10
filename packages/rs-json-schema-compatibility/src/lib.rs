mod change;
mod error;
mod keyword;

// TODO: Revisit visibility
use crate::change::{JsonSchemaChange, PatchOperationPath};
use crate::error::{
    Error, InvalidJsonPatchOperationPathError, UndefinedReplaceCallbackError,
    UnsupportedSchemaKeywordError,
};
pub use crate::keyword::*;
use json_patch::PatchOperation;
pub use json_patch::{AddOperation, RemoveOperation, ReplaceOperation};
use serde_json::Value;
use std::collections::HashSet;

struct RemovedItem {
    name: String,
    change: JsonSchemaChange,
}

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

#[derive(Default, Debug, Clone)]
pub struct ValidationOptions {
    pub allow_reorder: bool,
}

pub fn validate_schemas_compatibility(
    original_schema: &Value,
    new_schema: &Value,
) -> Result<CompatibilityValidationResult, Error> {
    validate_schema_compatibility_with_options(
        original_schema,
        new_schema,
        ValidationOptions::default(),
    )
}

pub fn validate_schema_compatibility_with_options(
    original_schema: &Value,
    new_schema: &Value,
    opts: ValidationOptions,
) -> Result<CompatibilityValidationResult, Error> {
    let patch = json_patch::diff(original_schema, new_schema);
    let mut incompatible_changes: Vec<JsonSchemaChange> = Vec::new();
    let mut removed: Vec<RemovedItem> = vec![];
    let mut inserted: HashSet<String> = HashSet::new();

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
            PatchOperation::Remove(_) => rule.allow_removing,

            PatchOperation::Replace(op) => {
                let callback = rule
                    .allow_replacing
                    .as_ref()
                    .ok_or(UndefinedReplaceCallbackError { keyword })?;

                callback(original_schema, op)?

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
                rule.allow_adding
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
        };

        if !is_compatible {
            incompatible_changes.push(operation.try_into()?);
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

//
// fn is_operation_replace_compatible(
//     path: &str,
//     previous_schema: &Value,
//     new_schema: &Value,
// ) -> Result<bool, Error> {
//     let Some((keyword, rule)) = find_keyword_rule(path)? else {
//         return Err(Error::InvalidJsonPatchOperationPath(
//             InvalidJsonPatchOperationPathError {
//                 path: path.to_string(),
//             },
//         ));
//     };
//
//     let pointer = Pointer::try_from(path).map_err(|error| InvalidJsonPointerPathError {
//         path: path.to_string(),
//         error,
//     })?;
//
//     let previous_value =
//         previous_schema
//             .resolve(&pointer)
//             .map_err(|error| JsonPointerPathNotFoundError {
//                 path: path.to_string(),
//                 error,
//             })?;
//
//     let new_value = new_schema
//         .resolve(&pointer)
//         .map_err(|error| JsonPointerPathNotFoundError {
//             path: path.to_string(),
//             error,
//         })?;
//
//     let callback = rule
//         .allow_replacing
//         .as_ref()
//         .ok_or(UndefinedReplaceCallbackError { keyword })?;
//
//     Ok(callback(previous_value, new_value))
// }

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

    // TODO: validate
    //  - Add more prefixItems and increase array size? Yes, we can
    //  - Modify subschema inside `items`, `prefixItems`
    //  - we should be able to reorder fields
    //  - what about $ref and $defs?

    #[test]
    fn test_find_rule() {
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
            .expect("should find keyword without failure");

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
