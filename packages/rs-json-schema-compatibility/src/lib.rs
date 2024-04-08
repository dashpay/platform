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
use json_patch::PatchOperation;
pub use json_patch::{AddOperation, RemoveOperation, ReplaceOperation};
use jsonptr::{Pointer, Resolve};
use serde_json::{Map as JsonMap, Value as JsonValue};

mod property_names {
    pub const REQUIRED: &str = "required";
    pub const DEFINITIONS: &str = "definitions";
    pub const PROPERTIES: &str = "properties";
    pub const REF: &str = "$ref";
    pub const MIN_ITEMS: &str = "minItems";
}

struct RemovedItem {
    name: String,
    change: JsonSchemaChange,
}

struct CompatibilityValidationResult {
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

pub fn validate_schema_compatibility(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
) -> Result<CompatibilityValidationResult, Error> {
    let patch = json_patch::diff(original_schema, new_schema);
    let mut incompatible_changes: Vec<JsonSchemaChange> = Vec::new();
    let mut removed: Vec<RemovedItem> = vec![];
    let mut inserted: HashSet<String> = HashSet::new();

    for operation in patch.0.into_iter() {
        match operation {
            PatchOperation::Remove(ref op) => {
                let keyword = get_last_sub_path(&op.path);

                if is_operation_remove_compatible(op_remove.path.as_str(), original_schema, &opts)?
                {
                    continue;
                }
                incompatible_changes.push(operation.try_into()?);
            }

            PatchOperation::Replace(ref op_replace) => {
                let is_min_items = is_min_items(&op_replace.path);
                let json_pointer =
                    Pointer::try_from(op_replace.path.as_str()).map_err(|error| {
                        InvalidJsonPointerPathError {
                            path: op_replace.path.clone(),
                            error,
                        }
                    })?;

                let old_value = original_schema.resolve(&json_pointer).map_err(|error| {
                    JsonPointerPathNotFoundError {
                        path: op_replace.path.clone(),
                        error,
                    }
                })?;

                if is_min_items && old_value.as_i64() > op_replace.value.as_i64() {
                    continue;
                }

                if !opts.allow_reorder {
                    incompatible_changes.push(operation.try_into()?);
                } else {
                    inserted.insert(op_replace.value.to_string());
                    removed.push(RemovedItem {
                        name: old_value.to_string(),
                        change: operation.try_into()?,
                    });
                }
            }
            PatchOperation::Add(ref op_add) => {
                let is_new_any_of_item = is_anyof_path(&op_add.path);
                let is_new_enum_value = is_enum_path(&op_add.path);
                let path_two_last_levels = get_second_last_sub_path(&op_add.path)
                    // TODO: No panic
                    .unwrap_or_else(|| {
                        panic!("the second subpath doesn't exist in '{}'", op_add.path)
                    });

                if path_two_last_levels == property_names::REQUIRED {
                    incompatible_changes.push(operation.try_into()?);
                    continue;
                }
                if [property_names::PROPERTIES, property_names::DEFINITIONS]
                    .contains(&path_two_last_levels)
                {
                    continue;
                }

                if is_new_any_of_item && opts.allow_reorder {
                    // TODO: Deal with this
                    inserted.insert(
                        op_add
                            .value
                            .get(property_names::REF)
                            .unwrap_or_else(|| &op_add.value)
                            // .ok_or(|| {
                            //     format!("the property '{}' doesn't exist", property_names::REF)
                            // })?
                            .to_string(),
                    );
                } else if (is_new_any_of_item && opts.allow_new_one_of)
                    || (is_new_enum_value && opts.allow_new_enum_value)
                {
                    continue;
                } else {
                    incompatible_changes.push(operation.try_into()?)
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
                Some(node.change)
            } else {
                None
            }
        });

        incompatible_changes.extend(filtered_removed);
    }

    Ok(CompatibilityValidationResult {
        incompatible_changes,
    })
}

// checks if operation `move` or `remove` is backward compatible
fn is_operation_remove_compatible(
    path: &str,
    original_schema: &JsonValue,
    opts: &ValidationOptions,
) -> Result<bool, Error> {
    let is_min_items = path.ends_with(property_names::MIN_ITEMS);
    if get_second_last_sub_path(path) == Some(property_names::REQUIRED) || is_min_items {
        return Ok(true);
    }

    // Check if the removed node is deprecated
    let is_any_of_item = is_anyof_path(path);
    if is_any_of_item {
        let json_pointer =
            Pointer::try_from(path).map_err(|error| InvalidJsonPointerPathError {
                path: path.to_string(),
                error,
            })?;

        let value = original_schema.resolve(&json_pointer).map_err(|error| {
            JsonPointerPathNotFoundError {
                path: path.to_string(),
                error,
            }
        })?;

        if let Some(ref_value) = value.get("$ref") {
            let ref_value_string = ref_value.to_string();

            // TODO: No panic
            let last_subpath = get_last_sub_path(&ref_value_string).unwrap_or_else(|| {
                panic!("The last subpath doesn't exist in '{}'", ref_value_string)
            });

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
    let mut path_segments = path.split('/');

    let Some(last_segment) = path_segments.next_back() else {
        return false;
    };

    if last_segment.parse::<usize>().is_err() {
        return false;
    }

    let Some(last_second_segment) = path_segments.next_back() else {
        return false;
    };

    if last_second_segment != path_type {
        return false;
    }

    true
}

fn get_second_last_sub_path(path: &str) -> Option<&str> {
    let mut path_segments = path.split('/');

    path_segments.next_back().and(path_segments.next_back())
}

fn get_last_sub_path(path: &str) -> Option<&str> {
    let mut path_segments = path.split('/');

    path_segments.next_back()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keywords::KeywordRule;
    use rstest::rstest;
    use serde_json::json;
    use test_case::{test_case, test_matrix};

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

    // TODO: validate
    //  - Add more prefixItems and increase array size?
    //  - Modify subschema inside `items`, `enum`, `dependentSchemas`, `prefixItems`
    //  - we should be able to reorder fields
    //  - what about $ref and $defs?

    #[rstest]
    // adding, removing, replacing, keyword, value, new_invalid_value, new_valid_value
    #[case::id(true, false, false, "$id", "foo", "bar", None)]
    #[case::ref_keyword(false, false, false, "$ref", "foo", "bar", None)]
    #[case::comment(true, true, true, "$comment", "foo", "bar", None)]
    #[case::description(true, true, true, "description", "foo", "bar", None)]
    #[case::examples(true, true, true, "examples", JsonValue::Array(vec![JsonValue::from("foo")]), JsonValue::Array(vec![JsonValue::from("foo")]), Some(JsonValue::Array(vec![JsonValue::from("foo"), JsonValue::from("boo")])))]
    #[case::multiple_of(false, false, false, "multipleOf", 123, 456, None)]
    #[case::maximum(false, true, true, "maximum", 123, 122, Some(124))]
    #[case::exclusive_maximum(false, true, true, "exclusiveMaximum", 123, 122, Some(124))]
    #[case::minimum(false, true, true, "minimum", 123, 124, Some(122))]
    #[case::exclusive_minimum(false, true, true, "exclusiveMinimum", 123, 124, Some(122))]
    #[case::max_length(false, true, true, "maxLength", 123, 122, Some(124))]
    #[case::min_length(false, true, true, "minLength", 123, 124, Some(122))]
    #[case::pattern(false, true, false, "pattern", "[a-z]", "[0-9]", None)]
    #[case::max_items(false, true, true, "maxItems", 123, 122, Some(124))]
    #[case::min_items(false, true, true, "minItems", 123, 124, Some(122))]
    #[case::unique_items(false, true, true, "uniqueItems", true, false, Some(true))]
    #[case::contains(false, true, false, "contains", JsonValue::Object(JsonMap::from_iter(vec![(String::from("type"), JsonValue::from("string"))])), JsonValue::Object(JsonMap::from_iter(vec![(String::from("type"), JsonValue::from("number"))])), None)]
    #[case::max_properties(false, true, true, "maxProperties", 123, 122, Some(124))]
    #[case::required(false, true, true, "required", JsonValue::Array(vec![JsonValue::from("property1")]), JsonValue::Array(vec![JsonValue::from("property1"), JsonValue::from("property2")]), Some(JsonValue::Array(Vec::new())))]
    #[case::additional_properties(false, false, false, "additionalProperties", true, false, None)]
    #[case::properties(false, false, false, "properties", JsonValue::Object(JsonMap::from_iter(vec![(String::from("property1"), JsonValue::Object(JsonMap::new()))])), JsonValue::Object(JsonMap::from_iter(vec![(String::from("property2"), JsonValue::Object(JsonMap::new()))])), None)]
    #[case::dependent_schemas(false, true, false, "dependentSchemas", JsonValue::Object(JsonMap::from_iter(vec![(String::from("property1"), JsonValue::Object(JsonMap::new()))])), JsonValue::Object(JsonMap::from_iter(vec![(String::from("property2"), JsonValue::Object(JsonMap::new()))])), None)]
    #[case::dependent_required(false, true, false, "dependentRequired", JsonValue::Object(JsonMap::from_iter(vec![(String::from("property1"), JsonValue::Array(Vec::new()))])), JsonValue::Object(JsonMap::from_iter(vec![(String::from("property2"), JsonValue::Array(Vec::new()))])), None)]
    #[case::const_keyword(false, true, false, "const", "foo", "boo", None)]
    #[case::enum_keyword(false, true, true, "enum", JsonValue::Array(vec![JsonValue::from(1), JsonValue::from(2)]), JsonValue::Array(vec![JsonValue::from(1)]), Some(JsonValue::Array(vec![JsonValue::from(1), JsonValue::from(2), JsonValue::from(3)])))]
    #[case::type_keyword(false, false, false, "type", "string", "object", None)]
    #[case::format(false, true, false, "format", "date", "time", None)]
    #[case::content_media_type(
        false,
        false,
        false,
        "contentMediaType",
        "application/x.dash.dpp.identifier",
        "application/unknown",
        None
    )]
    #[case::byte_array(false, false, false, "byteArray", true, false, None)]
    #[case::prefix_items(false, false, false, "prefixItems", JsonValue::Array(vec![]), JsonValue::Array(vec![JsonValue::Object(JsonMap::new())]), None)]
    #[case::items(false, false, false, "items", JsonValue::Object(JsonMap::from_iter(vec![(String::from("type"), JsonValue::from("string"))])), JsonValue::Object(JsonMap::from_iter(vec![(String::from("type"), JsonValue::from("object"))])), None)]
    #[case::position(false, false, false, "position", 0, 1, None)]
    fn test_schema_keyword_modification<V>(
        #[case] allow_adding: bool,
        #[case] allow_removing: bool,
        #[case] allow_replacing: bool,
        #[case] keyword: &str,
        #[case] value: V,
        #[case] new_invalid_value: V,
        #[case] new_valid_value: Option<V>,
    ) where
        V: Into<JsonValue> + Clone,
    {
        assert_adding(allow_adding, keyword, value.clone().into());

        assert_removing(allow_removing, keyword, value.clone().into());

        assert_replacing(
            allow_replacing,
            keyword,
            value.into(),
            new_invalid_value.into(),
            new_valid_value.map(|v| v.into()),
        );
    }

    fn assert_adding(allow: bool, keyword: &str, value: JsonValue) {
        let previous_schema = json!({});

        let next_schema = json!({
            keyword: value,
        });

        let result = validate_schemas_compatibility(&previous_schema, &next_schema)
            .expect("should not fail");

        if allow {
            assert!(
                result.is_compatible(),
                "assertion failed: adding of '{keyword}' is not allowed"
            );
        } else {
            let expected_path = format!("/{keyword}");
            assert!(
                matches!(
                    result.incompatible_changes(),
                    [JsonSchemaChange::Add(AddOperation { path, .. })] if path == &expected_path
                ),
                "assertion failed: adding of '{keyword}' returns {:?}\n expected: RemoveOperation {{ path: \"{expected_path}\" }}",
                result.incompatible_changes()
            );
        }
    }

    fn assert_removing(allow: bool, keyword: &str, value: JsonValue) {
        let previous_schema = json!({
            keyword: value,
        });

        let next_schema = json!({
            "type": "object",
        });

        let result = validate_schemas_compatibility(&previous_schema, &next_schema)
            .expect("should not fail");

        if allow {
            assert!(
                result.is_compatible(),
                "assertion failed: removing of '{keyword}' is not allowed"
            )
        } else {
            let expected_path = format!("/{keyword}");

            assert!(
                matches!(
                result.incompatible_changes(),
                [JsonSchemaChange::Remove(RemoveOperation { path })] if path == &expected_path),
                "assertion failed: removing of '{keyword}' returns {:?}\n expected: RemoveOperation {{ path: \"{expected_path}\" }}",
                result.incompatible_changes()
            );
        }
    }

    fn assert_replacing(
        allow: bool,
        keyword: &str,
        previous_value: JsonValue,
        new_invalid_value: JsonValue,
        new_valid_value: Option<JsonValue>,
    ) {
        let previous_schema = json!({
            keyword: previous_value.clone(),
        });

        let new_value = if allow {
            new_valid_value.expect("new_valid_value must be present if allow_replacing is true")
        } else {
            new_invalid_value.clone()
        };

        let new_schema = json!({
            keyword: new_value,
        });

        let result =
            validate_schemas_compatibility(&previous_schema, &new_schema).expect("should not fail");

        let expected_path = format!("/{keyword}");

        if allow {
            assert!(
                result.is_compatible(),
                "assertion failed: replacing of '{keyword}' is not allowed"
            );

            let previous_schema = json!({
                keyword: previous_value.clone(),
            });

            let new_schema = json!({
                keyword: new_invalid_value,
            });

            let result = validate_schemas_compatibility(&previous_schema, &new_schema)
                .expect("should not fail");

            assert!(
                matches!(
                    result.incompatible_changes(),
                    [JsonSchemaChange::Replace(ReplaceOperation { path, value })] if path == &expected_path && value == &new_invalid_value
                ),
                "assertion failed: replacing of '{keyword}' returns {:?}\n expected: ReplaceOperation {{ path: \"{expected_path}\", value: {:?} }}",
                result.incompatible_changes(),
                previous_value
            );
        } else {
            assert!(
                matches!(
                    result.incompatible_changes(),
                    [JsonSchemaChange::Replace(ReplaceOperation { path, value })] if path == &expected_path && value == &new_value
                ),
                "assertion failed: replacing of '{keyword}' returns {:?}\n expected: ReplaceOperation {{ path: \"{expected_path}\", value: {:?} }}",
                result.incompatible_changes(),
                previous_value
            );
        }
    }
}
