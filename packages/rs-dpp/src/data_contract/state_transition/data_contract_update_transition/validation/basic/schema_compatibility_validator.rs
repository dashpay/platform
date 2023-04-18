/// The Schema compatibility validator is a port of a JavaScript version
/// https://bitbucket.org/atlassian/json-schema-diff-validator/src/master/
///
/// The functionality has been ported 'as is' without any logic improvements and optimizations
use std::convert::TryFrom;

use anyhow::Context;
use itertools::Itertools;
use json_patch::PatchOperation;
use jsonptr::{Pointer, Resolve};
use serde_json::Value as JsonValue;
use thiserror::Error;

mod property_names {
    pub const REQUIRED: &str = "required";
    pub const DEFINITIONS: &str = "definitions";
    pub const PROPERTIES: &str = "properties";
    pub const REF: &str = "$ref";
    pub const MIN_ITEMS: &str = "minItems";
}

#[derive(Default, Debug, Clone)]
pub struct ValidationOptions {
    pub allow_new_one_of: bool,
    pub allow_new_enum_value: bool,
    pub allow_reorder: bool,
    pub deprecated_items: Vec<String>,
}

struct RemovedItem {
    name: String,
    operation: PatchOperation,
}

#[derive(Error, Debug)]
pub enum DiffVAlidatorError {
    /// The error that happens when there there is a problem with access to the fields
    /// in dynamic data structure
    #[error("Error while validation: {0}")]
    DataStructureError(#[from] anyhow::Error),
    /// The error is returned when validation proceeded but the schemas are not compatible
    #[error("The schemas are not compatible: {diffs:#?}")]
    SchemaCompatibilityError { diffs: Vec<PatchOperation> },
}

pub fn validate_schema_compatibility(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
) -> Result<(), DiffVAlidatorError> {
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
) -> Result<(), DiffVAlidatorError> {
    let patch = json_patch::diff(original_schema, new_schema);
    let mut diffs: Vec<PatchOperation> = vec![];
    let mut removed: Vec<RemovedItem> = vec![];
    let mut inserted: Vec<String> = vec![];

    for operation in patch.0.into_iter() {
        match operation {
            PatchOperation::Move(ref op_move) => {
                if is_operation_move_remove_compatible(
                    op_move.path.as_str(),
                    original_schema,
                    &opts,
                )? {
                    continue;
                }
                diffs.push(operation);
            }

            PatchOperation::Remove(ref op_remove) => {
                if is_operation_move_remove_compatible(
                    op_remove.path.as_str(),
                    original_schema,
                    &opts,
                )? {
                    continue;
                }
                diffs.push(operation);
            }

            PatchOperation::Replace(ref op_replace) => {
                let is_min_items = is_min_items(&op_replace.path);
                let json_pointer =
                    Pointer::try_from(op_replace.path.as_str()).with_context(|| {
                        format!("unable to create a json pointer from '{}'", op_replace.path)
                    })?;
                let old_value = original_schema.resolve(&json_pointer).with_context(|| {
                    format!("cannot find the '{}' in original schema", op_replace.path)
                })?;

                if is_min_items && old_value.as_i64() > op_replace.value.as_i64() {
                    continue;
                }

                if !opts.allow_reorder {
                    diffs.push(operation)
                } else {
                    inserted.push(op_replace.value.to_string());
                    removed.push(RemovedItem {
                        name: old_value.to_string(),
                        operation,
                    });
                }
            }
            PatchOperation::Add(ref op_add) => {
                let is_new_any_of_item = is_anyof_path(&op_add.path);
                let is_new_enum_value = is_enum_path(&op_add.path);
                let path_two_last_levels =
                    get_second_last_sub_path(&op_add.path).with_context(|| {
                        format!("the second subpath doesn't exist in '{}'", op_add.path)
                    })?;

                if path_two_last_levels == property_names::REQUIRED {
                    diffs.push(operation);
                    continue;
                }
                if [property_names::PROPERTIES, property_names::DEFINITIONS]
                    .contains(&path_two_last_levels)
                {
                    continue;
                }

                if is_new_any_of_item && opts.allow_reorder {
                    inserted.push(
                        op_add
                            .value
                            .get(property_names::REF)
                            .with_context(|| {
                                format!("the property '{}' doesn't exist", property_names::REF)
                            })?
                            .to_string(),
                    )
                } else if (is_new_any_of_item && opts.allow_new_one_of)
                    || (is_new_enum_value && opts.allow_new_enum_value)
                {
                    continue;
                } else {
                    diffs.push(operation)
                }
            }
            _ => continue,
        }
    }

    if opts.allow_reorder {
        // When reordering is allowed, we want ot make sure that any item that
        // was replaces is also inserted somewhere else
        let filtered_removed = removed.into_iter().filter_map(|node| {
            if inserted.contains(&node.name) {
                Some(node.operation)
            } else {
                None
            }
        });

        diffs.extend(filtered_removed);
    }

    if !diffs.is_empty() {
        return Err(DiffVAlidatorError::SchemaCompatibilityError { diffs });
    }

    Ok(())
}

// checks if operation `move` or `remove` is backward compatible
fn is_operation_move_remove_compatible(
    path: &str,
    original_schema: &JsonValue,
    opts: &ValidationOptions,
) -> Result<bool, anyhow::Error> {
    let is_min_items = path.ends_with(property_names::MIN_ITEMS);
    if get_second_last_sub_path(path) == Some(property_names::REQUIRED) || is_min_items {
        return Ok(true);
    }

    // Check if the removed node is deprecated
    let is_any_of_item = is_anyof_path(path);
    if is_any_of_item {
        let json_pointer: Pointer = Pointer::try_from(path)
            .with_context(|| format!("Unable to crate a Json Pointer from '{}'", path))?;
        let value = original_schema
            .resolve(&json_pointer)
            .with_context(|| format!("Cannot find the '{}' in the original schema", path))?;

        if let Some(ref_value) = value.get("$ref") {
            let ref_value_string = ref_value.to_string();
            let last_subpath = get_last_sub_path(&ref_value_string).with_context(|| {
                format!("The last subpath doesn't exist in '{}'", ref_value_string)
            })?;

            if opts.deprecated_items.iter().any(|i| i == last_subpath) {
                return Ok(true);
            }
        }
    } else {
        let last_subpath = get_last_sub_path(path).unwrap();
        if opts.deprecated_items.iter().any(|i| i == last_subpath) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn is_min_items(path: &str) -> bool {
    path.ends_with(property_names::MIN_ITEMS)
}

// checks if property path has form:  '.../anyOf/[usize]'
fn is_anyof_path(path: &str) -> bool {
    is_path_of_type(path, "anyOf")
}

// checks if property path has form:  '.../enum/[usize]'
fn is_enum_path(path: &str) -> bool {
    is_path_of_type(path, "enum")
}

// checks if property path has form:  '.../[name]/[usize]'
fn is_path_of_type(path: &str, path_type: &str) -> bool {
    let arr = path.split('/').collect_vec();
    if arr.len() < 2 {
        return false;
    }
    if arr[arr.len() - 1].parse::<usize>().is_err() {
        return false;
    }
    if arr[arr.len() - 2] != path_type {
        return false;
    }
    true
}

fn get_second_last_sub_path(path: &str) -> Option<&str> {
    let arr = path.split('/').collect_vec();
    if arr.len() > 1 {
        Some(arr[arr.len() - 2])
    } else {
        None
    }
}

fn get_last_sub_path(path: &str) -> Option<&str> {
    let arr = path.split('/').collect_vec();
    if !arr.is_empty() {
        Some(arr[arr.len() - 1])
    } else {
        None
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use lazy_static::lazy_static;
    use serde_json::json;

    lazy_static! {
        static ref DATA_SCHEMA: JsonValue = serde_json::from_str(include_str!(
            "./../../../../../tests/payloads/schema/data.json"
        ))
        .unwrap();
        static ref DATA_SCHEMA_V2: JsonValue = serde_json::from_str(include_str!(
            "./../../../../../tests/payloads/schema/data_v2.json"
        ))
        .unwrap();
    }

    #[test]
    fn test_is_any_of_item() {
        let any_of_item = "/anyOf/0";
        assert!(is_path_of_type(any_of_item, "anyOf"));

        let any_of_item = "/alpha/anyOf/0";
        assert!(is_path_of_type(any_of_item, "anyOf"));

        let is_not_any_of_item = "";
        assert!(!is_path_of_type(is_not_any_of_item, "anyOf"));

        let is_not_any_of_item = "/anyOf/o";
        assert!(!is_path_of_type(is_not_any_of_item, "anyOf"));

        let is_not_any_of_item = "/alpha/anyOf/o";
        assert!(!is_path_of_type(is_not_any_of_item, "anyOf"));

        let is_not_any_of_item = "/alpha/anyof/1";
        assert!(!is_path_of_type(is_not_any_of_item, "anyOf"));
    }

    #[test]
    fn should_return_ok_if_data_is_the_same() {
        let result = validate_schema_compatibility_with_options(
            &DATA_SCHEMA.clone(),
            &DATA_SCHEMA.clone(),
            ValidationOptions::default(),
        );
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn should_return_err_on_remove() {
        let result = validate_schema_compatibility_with_options(
            &DATA_SCHEMA.clone(),
            &DATA_SCHEMA_V2.clone(),
            ValidationOptions::default(),
        );
        assert!(matches!(
            result,
            Err(DiffVAlidatorError::SchemaCompatibilityError { .. })
        ));
    }

    #[test]
    fn should_return_ok_if_new_field_is_added_but_not_required() {
        let mut new_data_schema = DATA_SCHEMA.clone();
        new_data_schema["definitions"]["mntent"]["properties"]["field"] =
            json!({"type" : "number"});

        let result = validate_schema_compatibility_with_options(
            &DATA_SCHEMA.clone(),
            &new_data_schema,
            ValidationOptions::default(),
        );

        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn should_return_ok_if_field_becomes_optional() {
        let mut new_data_schema = DATA_SCHEMA.clone();
        new_data_schema[property_names::REQUIRED] = json!(["/"]);

        let result = validate_schema_compatibility_with_options(
            &DATA_SCHEMA.clone(),
            &new_data_schema,
            ValidationOptions::default(),
        );

        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn should_return_err_if_field_becomes_required() {
        let mut old_data_schema = DATA_SCHEMA.clone();
        old_data_schema[property_names::REQUIRED] = json!(["/"]);

        let result = validate_schema_compatibility_with_options(
            &old_data_schema,
            &DATA_SCHEMA.clone(),
            ValidationOptions::default(),
        );

        assert!(matches!(
            result,
            Err(DiffVAlidatorError::SchemaCompatibilityError { .. })
        ));
    }

    #[test]
    fn should_return_err_if_field_changes_its_type() {
        let mut new_data_schema = DATA_SCHEMA.clone();
        new_data_schema["definitions"]["mntent"] = json!({"type" : "number"});

        let result = validate_schema_compatibility_with_options(
            &DATA_SCHEMA.clone(),
            &new_data_schema,
            ValidationOptions::default(),
        );

        assert!(matches!(
            result,
            Err(DiffVAlidatorError::SchemaCompatibilityError { .. })
        ));
    }
}
