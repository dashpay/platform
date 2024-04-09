use std::collections::{BTreeMap, HashSet};
/// The Schema compatibility validator is a port of a JavaScript version
/// https://bitbucket.org/atlassian/json-schema-diff-validator/src/master/
///
/// The functionality has been ported 'as is' without any logic improvements and optimizations
use std::convert::TryFrom;

use crate::data_contract::DocumentName;
use crate::validation::SimpleValidationResult;
use crate::ProtocolError;
use anyhow::Context;
use itertools::Itertools;
use json_patch::PatchOperation;
use jsonptr::{Pointer, Resolve};
use lazy_static::lazy_static;
use serde_json::{json, Value as JsonValue};

lazy_static! {
    pub static ref EMPTY_JSON: JsonValue = json!({});
}

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
    error: IncompatibleSchemaChange,
}

pub fn any_schema_changes(
    old_schema: &BTreeMap<DocumentName, JsonValue>,
    new_schema: &JsonValue,
) -> bool {
    let changes = old_schema
        .iter()
        .filter(|(document_type, original_schema)| {
            let new_document_schema = new_schema.get(document_type).unwrap_or(&EMPTY_JSON);
            let diff = json_patch::diff(original_schema, new_document_schema);
            !diff.0.is_empty()
        })
        .count();

    changes > 0
}

pub(super) fn validate_schema_compatibility_v0(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
) -> Result<SimpleValidationResult<IncompatibleSchemaChange>, ProtocolError> {
    validate_schema_compatibility_with_options(
        original_schema,
        new_schema,
        ValidationOptions::default(),
    )
}

fn validate_schema_compatibility_with_options(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
    opts: ValidationOptions,
) -> Result<SimpleValidationResult<IncompatibleSchemaChange>, ProtocolError> {
    let patch = json_patch::diff(original_schema, new_schema);
    let mut diffs: Vec<IncompatibleSchemaChange> = Vec::new();
    let mut removed: Vec<RemovedItem> = vec![];
    let mut inserted: HashSet<String> = HashSet::new();

    for operation in patch.0.into_iter() {
        match operation {
            PatchOperation::Remove(ref op_remove) => {
                if is_operation_remove_compatible(op_remove.path.as_str(), original_schema, &opts)?
                {
                    continue;
                }
                diffs.push(operation.try_into()?);
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
                    diffs.push(operation.try_into()?);
                } else {
                    inserted.insert(op_replace.value.to_string());
                    removed.push(RemovedItem {
                        name: old_value.to_string(),
                        error: operation.try_into()?,
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
                    diffs.push(operation.try_into()?);
                    continue;
                }
                if [property_names::PROPERTIES, property_names::DEFINITIONS]
                    .contains(&path_two_last_levels)
                {
                    continue;
                }

                if is_new_any_of_item && opts.allow_reorder {
                    inserted.insert(
                        op_add
                            .value
                            .get(property_names::REF)
                            .with_context(|| {
                                format!("the property '{}' doesn't exist", property_names::REF)
                            })?
                            .to_string(),
                    );
                } else if (is_new_any_of_item && opts.allow_new_one_of)
                    || (is_new_enum_value && opts.allow_new_enum_value)
                {
                    continue;
                } else {
                    diffs.push(operation.try_into()?)
                }
            }
            PatchOperation::Test(_) | PatchOperation::Copy(_) | PatchOperation::Move(_) => {
                unreachable!(
                    "json_patch diff doesn't return decorative operations test, copy, move"
                )
            }
        }
    }

    if opts.allow_reorder {
        // When reordering is allowed, we want ot make sure that any item that
        // was replaces is also inserted somewhere else
        let filtered_removed = removed.into_iter().filter_map(|node| {
            if inserted.contains(&node.name) {
                Some(node.error)
            } else {
                None
            }
        });

        diffs.extend(filtered_removed);
    }

    Ok(SimpleValidationResult::new_with_errors(diffs))
}

// checks if operation `move` or `remove` is backward compatible
fn is_operation_remove_compatible(
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
    use json_patch::RemoveOperation;
    use serde_json::json;

    // TODO: We don't need this
    use crate::consensus::basic::BasicError::IncompatibleDataContractSchemaError;
    use crate::data_contract::document_type::schema::IncompatibleSchemaChange;
    use lazy_static::lazy_static;
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

    // TODO: We can remove from required
    // TODO: We can:
    //  1. increase or remove maximum and exclusiveMaximum
    //  2. decrease or remove minimum and exclusiveMinimum
    //  3. increase or remove maxLength
    //  4. decrease or remove minLength
    //  5. increase or remove maxItems
    //  6. decrease or remove minItems
    //  7. remove uniqueItems
    //  8. remove contains
    //  9. increase or remove maxProperties
    //  10. decrease or remove minProperties
    //  11. remove from required except createdAt and updatedAt
    //  12. add new properties
    //  13. do not allow to add new properties to dependentSchemas, but allow to remove properties. The same recursive rules for subschemas
    //  14. do not allow to add to dependentRequired but allow to remove properties and elements of arrays
    //  15. Remove const
    //  17. Remove enum and add new items
    //  18. we can remove format
    //  19. Remove prefixItems or remove items from the end of the array
    //  20. Remove `items`, modify subschema the same way
    //  21. we should be able to reorder items
    //  22. what about $ref and $defs?
    // 23. we can change $comment, example, description

    mod id {
        use super::*;

        #[test]
        fn should_allow_to_add() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$id": "123",
                    }
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(result.is_valid());
        }

        #[test]
        fn should_deny_to_remove() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$id": "123",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string"
                    }
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(!result.is_valid());

            assert!(matches!(
                result.errors.as_slice(),
                [IncompatibleSchemaChange { name, path }] if name == "remove" && path == "/properties/field/$id"
            ));
        }

        #[test]
        fn should_deny_to_replace() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$id": "123",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$id": "456",
                    },
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(!result.is_valid());

            assert!(matches!(
                result.errors.as_slice(),
                [IncompatibleSchemaChange { name, path }] if name == "replace" && path == "/properties/field/$id"
            ));
        }
    }

    mod r#ref {
        use super::*;

        #[test]
        fn should_deny_to_add() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$ref": "/123",
                    }
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(!result.is_valid());

            assert!(matches!(
                result.errors.as_slice(),
                [IncompatibleSchemaChange { name, path }] if name == "add" && path == "/properties/field/$ref"
            ));
        }

        #[test]
        fn should_deny_to_remove() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$ref": "/123",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string"
                    }
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(!result.is_valid());

            assert!(matches!(
                result.errors.as_slice(),
                [IncompatibleSchemaChange { name, path }] if name == "remove" && path == "/properties/field/$ref"
            ));
        }

        #[test]
        fn should_deny_to_replace() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$ref": "/123",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$ref": "/456",
                    },
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(!result.is_valid());

            assert!(matches!(
                result.errors.as_slice(),
                [IncompatibleSchemaChange { name, path }] if name == "replace" && path == "/properties/field/$ref"
            ));
        }
    }

    mod comment {
        use super::*;

        #[test]
        fn should_allow_to_add() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$comment": "123",
                    }
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(result.is_valid());
        }

        #[test]
        fn should_allow_to_remove() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$comment": "123",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                    }
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(result.is_valid());
        }

        #[test]
        fn should_allow_to_replace() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$comment": "123",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "$comment": "456",
                    }
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(result.is_valid());
        }
    }

    mod description {
        use super::*;

        #[test]
        fn should_allow_to_add() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "description": "123",
                    }
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(result.is_valid());
        }

        #[test]
        fn should_allow_to_remove() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "description": "123",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                    }
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(result.is_valid());
        }

        #[test]
        fn should_allow_to_replace() {
            let previous_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "description": "123",
                    },
                }
            });

            let next_schema = json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "description": "456",
                    }
                }
            });

            let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
                .expect("should not fail");

            assert!(result.is_valid());
        }
    }

    #[test]
    fn should_return_error_if_property_is_removed() {
        let previous_schema = json!({
            "type": "object",
            "properties": {
                "field": {
                    "type": "string"
                },
                "field2": {
                    "type": "string"
                }
            }
        });

        let next_schema = json!({
            "type": "object",
            "properties": {
                "field": {
                    "type": "string"
                }
            }
        });

        let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
            .expect("should not fail");

        assert!(!result.is_valid());

        assert!(matches!(
            result.errors.as_slice(),
            [IncompatibleSchemaChange { name, path }] if name == "remove" && path == "/properties/field2"
        ));
    }

    #[test]
    fn should_return_ok_if_property_is_added() {
        let previous_schema = json!({
            "type": "object",
            "properties": {
                "field": {
                    "type": "string"
                }
            }
        });

        let next_schema = json!({
            "type": "object",
            "properties": {
                "field": {
                    "type": "string"
                },
                "field2": {
                    "type": "string"
                }
            }
        });

        let result = validate_schema_compatibility_v0(&previous_schema, &next_schema)
            .expect("should not fail");

        assert!(result.is_valid());
    }

    // #[test]
    // fn should_return_ok_if_data_is_the_same() {
    //     let result = validate_schema_compatibility_with_options(
    //         &DATA_SCHEMA.clone(),
    //         &DATA_SCHEMA.clone(),
    //         ValidationOptions::default(),
    //     );
    //     assert!(matches!(result, Ok(operations) if operations.is_empty()));
    // }
    //
    // #[test]
    // fn should_return_err_on_remove() {
    //     let result = validate_schema_compatibility_with_options(
    //         &DATA_SCHEMA.clone(),
    //         &DATA_SCHEMA_V2.clone(),
    //         ValidationOptions::default(),
    //     );
    //     assert!(matches!(
    //         result,
    //         Ok(operations) if operations.len() == 1
    //     ));
    // }
    //
    // #[test]
    // fn should_return_ok_if_new_field_is_added_but_not_required() {
    //     let mut new_data_schema = DATA_SCHEMA.clone();
    //     new_data_schema["definitions"]["mntent"]["properties"]["field"] =
    //         json!({"type" : "number"});
    //
    //     let result = validate_schema_compatibility_with_options(
    //         &DATA_SCHEMA.clone(),
    //         &new_data_schema,
    //         ValidationOptions::default(),
    //     );
    //
    //     assert!(matches!(result, Ok(operations) if operations.is_empty()));
    // }
    //
    // #[test]
    // fn should_return_ok_if_field_becomes_optional() {
    //     let mut new_data_schema = DATA_SCHEMA.clone();
    //     new_data_schema[property_names::REQUIRED] = json!(["/"]);
    //
    //     let result = validate_schema_compatibility_with_options(
    //         &DATA_SCHEMA.clone(),
    //         &new_data_schema,
    //         ValidationOptions::default(),
    //     );
    //
    //     assert!(matches!(result, Ok(operations) if operations.is_empty()));
    // }
    //
    // // TODO: remove min items
    // // TODO: reorder items
    //
    // #[test]
    // fn should_return_err_if_field_becomes_required() {
    //     let mut old_data_schema = DATA_SCHEMA.clone();
    //     old_data_schema[property_names::REQUIRED] = json!(["/"]);
    //
    //     let result = validate_schema_compatibility_with_options(
    //         &old_data_schema,
    //         &DATA_SCHEMA.clone(),
    //         ValidationOptions::default(),
    //     );
    //
    //     assert!(matches!(
    //         result,
    //         Ok(operations) if operations.len() == 1
    //     ));
    // }
    //
    // #[test]
    // fn should_return_err_if_field_changes_its_type() {
    //     let mut new_data_schema = DATA_SCHEMA.clone();
    //     new_data_schema["definitions"]["mntent"] = json!({"type" : "number"});
    //
    //     let result = validate_schema_compatibility_with_options(
    //         &DATA_SCHEMA.clone(),
    //         &new_data_schema,
    //         ValidationOptions::default(),
    //     );
    //
    //     assert!(matches!(
    //         result,
    //         Ok(operations) if !operations.is_empty()
    //     ));
    // }
}
