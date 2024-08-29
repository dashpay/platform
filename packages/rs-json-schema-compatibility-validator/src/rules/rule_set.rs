use super::compatibility_rules::CompatibilityRules;
use super::value::ValueTryMethods;
use super::IsReplacementAllowedCallback;
#[cfg(any(test, feature = "examples"))]
use crate::change::JsonSchemaChange;
use crate::error::{Error, InvalidJsonPatchOperationPathError, UnexpectedJsonValueTypeError};
#[cfg(any(test, feature = "examples"))]
use json_patch::{AddOperation, RemoveOperation, ReplaceOperation};
use once_cell::sync::Lazy;
#[cfg(any(test, feature = "examples"))]
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Replacement is never allowed
static FALSE_CALLBACK: Lazy<IsReplacementAllowedCallback> =
    Lazy::new(|| Some(Arc::new(|_, _| Ok(false))));

/// Replacement is always allowed
static TRUE_CALLBACK: Lazy<IsReplacementAllowedCallback> =
    Lazy::new(|| Some(Arc::new(|_, _| Ok(true))));

/// Replacement is allowed if a new u64 value is bigger than original
static U64_BIGGER_CALLBACK: Lazy<IsReplacementAllowedCallback> = Lazy::new(|| {
    Some(Arc::new(|schema, op| {
        let original_value = schema.try_pointer(&op.path)?.try_to_u64()?;
        let new_value = op.value.try_to_u64()?;

        Ok(original_value < new_value)
    }))
});

/// Replacement is allowed if a new u64 value is smaller than original
static U64_SMALLER_CALLBACK: Lazy<IsReplacementAllowedCallback> = Lazy::new(|| {
    Some(Arc::new(|schema, op| {
        let original_value = schema.try_pointer(&op.path)?.try_to_u64()?;
        let new_value = op.value.try_to_u64()?;

        Ok(original_value > new_value)
    }))
});

/// Replacement is allowed if a new f64 value is bigger than original
static F64_BIGGER_CALLBACK: Lazy<IsReplacementAllowedCallback> = Lazy::new(|| {
    Some(Arc::new(|schema, op| {
        let original_value = schema.try_pointer(&op.path)?.try_to_f64()?;
        let new_value = op.value.try_to_f64()?;

        Ok(original_value < new_value)
    }))
});

/// Replacement is allowed if a new f64 value is smaller than original
static F64_SMALLER_CALLBACK: Lazy<IsReplacementAllowedCallback> = Lazy::new(|| {
    Some(Arc::new(|schema, op| {
        let original_value = schema.try_pointer(&op.path)?.try_to_f64()?;
        let new_value = op.value.try_to_f64()?;

        Ok(original_value > new_value)
    }))
});

/// Replacement is allowed if a new value is an existing element in the original array
static EXISTING_ELEMENT_CALLBACK: Lazy<IsReplacementAllowedCallback> = Lazy::new(|| {
    Some(Arc::new(|schema, op| {
        // One segment back to required array
        let path = PathBuf::from(&op.path);
        let required_path = path.parent().and_then(|p| p.to_str()).ok_or_else(|| {
            InvalidJsonPatchOperationPathError {
                path: op.path.clone(),
            }
        })?;

        let original_required_value = schema.try_pointer(required_path)?;

        let original_required_array_of_values =
            original_required_value.as_array().ok_or_else(|| {
                Error::UnexpectedJsonValueType(UnexpectedJsonValueTypeError {
                    expected_type: "array".to_string(),
                    value: original_required_value.clone(),
                })
            })?;

        Ok(original_required_array_of_values.contains(&op.value))
    }))
});

pub type CompatibilityRulesCollection = HashMap<&'static str, CompatibilityRules>;

/// The rules define, which change in JSON Schema for a keyword and its inner structure is compatible or not.
/// Important note: Not all keywords are supported, and some rules are
/// implemented based on Data Contract schema validation requirements
pub static KEYWORD_COMPATIBILITY_RULES: Lazy<CompatibilityRulesCollection> = Lazy::new(|| {
    HashMap::from_iter([
        (
            "$id",
            CompatibilityRules {
                allow_addition: true,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (json!({}), json!({ "$id": "foo" }), None).into(),
                    (
                        json!({ "$id": "foo" }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/$id".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "$id": "foo" }),
                        json!({ "$id": "bar" }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/$id".to_string(),
                            value: json!("bar"),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "$ref",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "$ref": "/foo" }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/$ref".to_string(),
                            value: json!("/foo"),
                        })),
                    )
                        .into(),
                    (
                        json!({ "$ref": "/foo" }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/$ref".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "$ref": "/foo" }),
                        json!({ "$ref": "/bar" }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/$ref".to_string(),
                            value: json!("/bar"),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "$comment",
            CompatibilityRules {
                allow_addition: true,
                allow_removal: true,
                allow_replacement_callback: TRUE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (json!({}), json!({ "$comment": "foo" }), None).into(),
                    (json!({ "$comment": "foo" }), json!({}), None).into(),
                    (
                        json!({ "$comment": "foo" }),
                        json!({ "$comment": "bar" }),
                        None,
                    )
                        .into(),
                ],
            },
        ),
        (
            "description",
            CompatibilityRules {
                allow_addition: true,
                allow_removal: true,
                allow_replacement_callback: TRUE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (json!({}), json!({ "description": "foo" }), None).into(),
                    (json!({ "description": "foo" }), json!({}), None).into(),
                    (
                        json!({ "description": "foo" }),
                        json!({ "description": "bar" }),
                        None,
                    )
                        .into(),
                ],
            },
        ),
        (
            "examples",
            CompatibilityRules {
                allow_addition: true,
                allow_removal: true,
                allow_replacement_callback: TRUE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (json!({}), json!({ "examples": ["foo"] }), None).into(),
                    (json!({ "examples": ["foo"] }), json!({}), None).into(),
                    (
                        json!({ "examples": ["foo"] }),
                        json!({ "examples": ["foo","bar"] }),
                        None,
                    )
                        .into(),
                    (
                        json!({ "examples": ["foo","bar"] }),
                        json!({ "examples": ["foo"] }),
                        None,
                    )
                        .into(),
                    (
                        json!({ "examples": ["foo"] }),
                        json!({ "examples": ["bar"] }),
                        None,
                    )
                        .into(),
                ],
            },
        ),
        (
            "multipleOf",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "multipleOf": 1.0 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/multipleOf".to_string(),
                            value: json!(1.0),
                        })),
                    )
                        .into(),
                    (
                        json!({ "multipleOf": 1.0 }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/multipleOf".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "multipleOf": 1.0 }),
                        json!({ "multipleOf": 2.0 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/multipleOf".to_string(),
                            value: json!(2.0),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "maximum",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: F64_BIGGER_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "maximum": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/maximum".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (json!({ "maximum": 1 }), json!({}), None).into(),
                    (json!({ "maximum": 1 }), json!({ "maximum": 2 }), None).into(),
                    (
                        json!({ "maximum": 2 }),
                        json!({ "maximum": 1 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/maximum".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "exclusiveMaximum",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: F64_BIGGER_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "exclusiveMaximum": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/exclusiveMaximum".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (json!({ "exclusiveMaximum": 1 }), json!({}), None).into(),
                    (
                        json!({ "exclusiveMaximum": 1 }),
                        json!({ "exclusiveMaximum": 2 }),
                        None,
                    )
                        .into(),
                    (
                        json!({ "exclusiveMaximum": 2 }),
                        json!({ "exclusiveMaximum": 1 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/exclusiveMaximum".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "minimum",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: F64_SMALLER_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "minimum": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/minimum".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (json!({ "minimum": 1 }), json!({}), None).into(),
                    (
                        json!({ "minimum": 1 }),
                        json!({ "minimum": 2 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/minimum".to_string(),
                            value: json!(2),
                        })),
                    )
                        .into(),
                    (json!({ "minimum": 2 }), json!({ "minimum": 1 }), None).into(),
                ],
            },
        ),
        (
            "exclusiveMinimum",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: F64_SMALLER_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "exclusiveMinimum": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/exclusiveMinimum".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (json!({ "exclusiveMinimum": 1 }), json!({}), None).into(),
                    (
                        json!({ "exclusiveMinimum": 1 }),
                        json!({ "exclusiveMinimum": 2 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/exclusiveMinimum".to_string(),
                            value: json!(2),
                        })),
                    )
                        .into(),
                    (
                        json!({ "exclusiveMinimum": 2 }),
                        json!({ "exclusiveMinimum": 1 }),
                        None,
                    )
                        .into(),
                ],
            },
        ),
        (
            "maxLength",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: U64_BIGGER_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "maxLength": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/maxLength".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (json!({ "maxLength": 1 }), json!({}), None).into(),
                    (json!({ "maxLength": 1 }), json!({ "maxLength": 2 }), None).into(),
                    (
                        json!({ "maxLength": 2 }),
                        json!({ "maxLength": 1 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/maxLength".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "minLength",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: U64_SMALLER_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "minLength": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/minLength".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (json!({ "minLength": 1 }), json!({}), None).into(),
                    (
                        json!({ "minLength": 1 }),
                        json!({ "minLength": 2 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/minLength".to_string(),
                            value: json!(2),
                        })),
                    )
                        .into(),
                    (json!({ "minLength": 2 }), json!({ "minLength": 1 }), None).into(),
                ],
            },
        ),
        (
            "pattern",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "pattern": "[a-z]" }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/pattern".to_string(),
                            value: json!("[a-z]"),
                        })),
                    )
                        .into(),
                    (json!({ "pattern": "[a-z]" }), json!({}), None).into(),
                    (
                        json!({ "pattern": "[a-z]" }),
                        json!({ "pattern": "[0-9]" }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/pattern".to_string(),
                            value: json!("[0-9]"),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "maxItems",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: U64_BIGGER_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "maxItems": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/maxItems".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (json!({ "maxItems": 1 }), json!({}), None).into(),
                    (json!({ "maxItems": 1 }), json!({ "maxItems": 2 }), None).into(),
                    (
                        json!({ "maxItems": 2 }),
                        json!({ "maxItems": 1 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/maxItems".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "minItems",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: U64_SMALLER_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "minItems": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/minItems".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (json!({ "minItems": 1 }), json!({}), None).into(),
                    (
                        json!({ "minItems": 1 }),
                        json!({ "minItems": 2 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/minItems".to_string(),
                            value: json!(2),
                        })),
                    )
                        .into(),
                    (json!({ "minItems": 2 }), json!({ "minItems": 1 }), None).into(),
                ],
            },
        ),
        (
            "uniqueItems",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: Some(Arc::new(|schema, op| {
                    let original_value = schema.try_pointer(&op.path)?.try_to_bool()?;
                    let new_value = op.value.try_to_bool()?;

                    Ok(original_value && !new_value)
                })),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "uniqueItems": true }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/uniqueItems".to_string(),
                            value: json!(true),
                        })),
                    )
                        .into(),
                    (json!({ "uniqueItems": true }), json!({}), None).into(),
                    (json!({ "uniqueItems": false }), json!({}), None).into(),
                    (
                        json!({ "uniqueItems": false }),
                        json!({ "uniqueItems": true }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/uniqueItems".to_string(),
                            value: json!(true),
                        })),
                    )
                        .into(),
                    (
                        json!({ "uniqueItems": true }),
                        json!({ "uniqueItems": false }),
                        None,
                    )
                        .into(),
                ],
            },
        ),
        (
            "required",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: None,
                subschema_levels_depth: None,
                inner: Some(Box::new(CompatibilityRules {
                    allow_addition: false,
                    allow_removal: true,
                    allow_replacement_callback: EXISTING_ELEMENT_CALLBACK.clone(),
                    subschema_levels_depth: None,
                    inner: None,
                    #[cfg(any(test, feature = "examples"))]
                    examples: vec![
                        (
                            json!({ "required": ["foo"] }),
                            json!({ "required": ["foo", "bar"] }),
                            Some(JsonSchemaChange::Add(AddOperation {
                                path: "/required/1".to_string(),
                                value: json!("bar"),
                            })),
                        )
                            .into(),
                        (
                            json!({ "required": ["foo", "bar"] }),
                            json!({ "required": ["foo"] }),
                            None,
                        )
                            .into(),
                        (
                            json!({ "required": ["foo", "bar"] }),
                            json!({ "required": ["bar"] }),
                            None,
                        )
                            .into(),
                        (
                            json!({ "required": ["foo"] }),
                            json!({ "required": ["bar"] }),
                            Some(JsonSchemaChange::Replace(ReplaceOperation {
                                path: "/required/0".to_string(),
                                value: json!("bar"),
                            })),
                        )
                            .into(),
                        (
                            json!({ "required": ["foo", "bar"] }),
                            json!({ "required": ["bar", "foo"] }),
                            None,
                        )
                            .into(),
                    ],
                })),
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "required": ["foo"] }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/required".to_string(),
                            value: json!(["foo"]),
                        })),
                    )
                        .into(),
                    (json!({ "required": ["foo"] }), json!({}), None).into(),
                ],
            },
        ),
        (
            "properties",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: None,
                subschema_levels_depth: Some(2),
                inner: Some(Box::new(CompatibilityRules {
                    allow_addition: true,
                    allow_removal: false,
                    allow_replacement_callback: None,
                    subschema_levels_depth: None,
                    inner: None,
                    #[cfg(any(test, feature = "examples"))]
                    examples: vec![
                        (
                            json!({ "properties": {"foo": {}} }),
                            json!({ "properties": {"foo": {}, "bar": {}} }),
                            None,
                        )
                            .into(),
                        (
                            json!({ "properties": {"foo": {}, "bar": {}} }),
                            json!({ "properties": {"foo": {}} }),
                            Some(JsonSchemaChange::Remove(RemoveOperation {
                                path: "/properties/bar".to_string(),
                            })),
                        )
                            .into(),
                        (
                            json!({ "properties": {"foo": {}} }),
                            json!({ "properties": {"foo": {}, "type": {}} }),
                            None,
                        )
                            .into(),
                    ],
                })),
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "properties": {"foo": {}} }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/properties".to_string(),
                            value: json!({"foo": {}} ),
                        })),
                    )
                        .into(),
                    (
                        json!({ "properties": {"foo": {}} }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/properties".to_string(),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "additionalProperties",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "additionalProperties": false }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/additionalProperties".to_string(),
                            value: json!(false),
                        })),
                    )
                        .into(),
                    (
                        json!({ "additionalProperties": false }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/additionalProperties".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "additionalProperties": false }),
                        json!({ "additionalProperties": true }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/additionalProperties".to_string(),
                            value: json!(true),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "dependentRequired",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: None,
                subschema_levels_depth: None,
                inner: Some(Box::new(CompatibilityRules {
                    allow_addition: false,
                    allow_removal: true,
                    allow_replacement_callback: EXISTING_ELEMENT_CALLBACK.clone(),
                    subschema_levels_depth: None,
                    inner: None,
                    #[cfg(any(test, feature = "examples"))]
                    examples: vec![
                        (
                            json!({ "dependentRequired": {"foo": []} }),
                            json!({ "dependentRequired": {"foo": [], "bar": []} }),
                            Some(JsonSchemaChange::Add(AddOperation {
                                path: "/dependentRequired/bar".to_string(),
                                value: json!([]),
                            })),
                        )
                            .into(),
                        (
                            json!({ "dependentRequired": {"foo": [], "bar": []} }),
                            json!({ "dependentRequired": {"foo": []} }),
                            None,
                        )
                            .into(),
                        (
                            json!({ "dependentRequired": {"foo": ["bar"]} }),
                            json!({ "dependentRequired": {"foo": ["bar", "baz"]} }),
                            Some(JsonSchemaChange::Add(AddOperation {
                                path: "/dependentRequired/foo/1".to_string(),
                                value: json!("baz"),
                            })),
                        )
                            .into(),
                        (
                            json!({ "dependentRequired": {"foo": ["bar", "baz"]} }),
                            json!({ "dependentRequired": {"foo": ["bar"]} }),
                            None,
                        )
                            .into(),
                        (
                            json!({ "dependentRequired": {"foo": ["bar", "baz"]} }),
                            json!({ "dependentRequired": {"foo": ["baz"]} }),
                            None,
                        )
                            .into(),
                        (
                            json!({ "dependentRequired": {"foo": ["bar"]} }),
                            json!({ "dependentRequired": {"foo": ["baz"]} }),
                            Some(JsonSchemaChange::Replace(ReplaceOperation {
                                path: "/dependentRequired/foo/0".to_string(),
                                value: json!("baz"),
                            })),
                        )
                            .into(),
                    ],
                })),
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "dependentRequired": {"foo": ["bar"]} }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/dependentRequired".to_string(),
                            value: json!({"foo": ["bar"]}),
                        })),
                    )
                        .into(),
                    (
                        json!({ "dependentRequired": {"foo": ["bar"]} }),
                        json!({}),
                        None,
                    )
                        .into(),
                ],
            },
        ),
        (
            "const",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: Some(Box::new(CompatibilityRules {
                    allow_addition: false,
                    allow_removal: false,
                    allow_replacement_callback: FALSE_CALLBACK.clone(),
                    subschema_levels_depth: None,
                    inner: None,
                    #[cfg(any(test, feature = "examples"))]
                    examples: vec![
                        (
                            json!({ "const": { "property": { "inner": true } }}),
                            json!({ "const": { "property": { "inner": true, "second": true } }}),
                            Some(JsonSchemaChange::Add(AddOperation {
                                path: "/const/property/second".to_string(),
                                value: json!(true),
                            })),
                        )
                            .into(),
                        (json!({ "const": "foo" }), json!({}), None).into(),
                        (
                            json!({ "const": { "property": { "inner": true, "second": true } }}),
                            json!({ "const": { "property": { "inner": true } }}),
                            Some(JsonSchemaChange::Remove(RemoveOperation {
                                path: "/const/property/second".to_string(),
                            })),
                        )
                            .into(),
                        (
                            json!({ "const": [ "item1" ]}),
                            json!({ "const": [ "item1", "item2" ]}),
                            Some(JsonSchemaChange::Add(AddOperation {
                                path: "/const/1".to_string(),
                                value: json!("item2"),
                            })),
                        )
                            .into(),
                        (
                            json!({ "const": [ "item1", "item2" ]}),
                            json!({ "const": [ "item1" ]}),
                            Some(JsonSchemaChange::Remove(RemoveOperation {
                                path: "/const/1".to_string(),
                            })),
                        )
                            .into(),
                        (
                            json!({ "const": [ "item1" ]}),
                            json!({ "const": [ "item2" ]}),
                            Some(JsonSchemaChange::Replace(ReplaceOperation {
                                path: "/const/0".to_string(),
                                value: json!("item2"),
                            })),
                        )
                            .into(),
                    ],
                })),
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "const": "foo" }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/const".to_string(),
                            value: json!("foo"),
                        })),
                    )
                        .into(),
                    (json!({ "const": "foo" }), json!({}), None).into(),
                    (
                        json!({ "const": "foo" }),
                        json!({ "const": "bar" }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/const".to_string(),
                            value: json!("bar"),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "enum",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: None,
                subschema_levels_depth: None,
                inner: Some(Box::new(CompatibilityRules {
                    allow_addition: true,
                    allow_removal: false,
                    allow_replacement_callback: EXISTING_ELEMENT_CALLBACK.clone(),
                    subschema_levels_depth: None,
                    inner: Some(Box::new(CompatibilityRules {
                        allow_addition: false,
                        allow_removal: false,
                        allow_replacement_callback: FALSE_CALLBACK.clone(),
                        subschema_levels_depth: None,
                        inner: None,
                        #[cfg(any(test, feature = "examples"))]
                        examples: vec![
                            (
                                json!({ "enum": [{ "property": ["foo"]}] }),
                                json!({ "enum": [{ "property": ["foo", "bar"]}] }),
                                Some(JsonSchemaChange::Add(AddOperation {
                                    path: "/enum/0/property/1".to_string(),
                                    value: json!("bar"),
                                })),
                            )
                                .into(),
                            (
                                json!({ "enum": [{ "property": ["foo", "bar"]}] }),
                                json!({ "enum": [{ "property": ["foo"]}] }),
                                Some(JsonSchemaChange::Remove(RemoveOperation {
                                    path: "/enum/0/property/bar".to_string(),
                                })),
                            )
                                .into(),
                            (
                                json!({ "enum": [{ "property": ["foo"]}] }),
                                json!({ "enum": [{ "property": ["bar"]}] }),
                                Some(JsonSchemaChange::Replace(ReplaceOperation {
                                    path: "/enum/0/property/0".to_string(),
                                    value: json!("bar"),
                                })),
                            )
                                .into(),
                        ],
                    })),
                    #[cfg(any(test, feature = "examples"))]
                    examples: vec![
                        (
                            json!({ "enum": ["foo"] }),
                            json!({ "enum": ["foo", "bar"] }),
                            None,
                        )
                            .into(),
                        (
                            json!({ "enum": ["foo", "bar"] }),
                            json!({ "enum": ["foo"] }),
                            Some(JsonSchemaChange::Remove(RemoveOperation {
                                path: "/enum/1".to_string(),
                            })),
                        )
                            .into(),
                        (
                            json!({ "enum": ["foo"] }),
                            json!({ "enum": ["bar"] }),
                            Some(JsonSchemaChange::Replace(ReplaceOperation {
                                path: "/enum/0".to_string(),
                                value: json!("bar"),
                            })),
                        )
                            .into(),
                    ],
                })),
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "enum": ["foo"] }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/enum".to_string(),
                            value: json!(["foo"]),
                        })),
                    )
                        .into(),
                    (json!({ "enum": ["foo"] }), json!({}), None).into(),
                ],
            },
        ),
        (
            "type",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "type": "string" }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/type".to_string(),
                            value: json!("string"),
                        })),
                    )
                        .into(),
                    (
                        json!({ "type": "string" }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/type".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "type": "string" }),
                        json!({ "type": "object" }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/type".to_string(),
                            value: json!("object"),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "format",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: true,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "format": "date" }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/format".to_string(),
                            value: json!("date"),
                        })),
                    )
                        .into(),
                    (json!({ "format": "date" }), json!({}), None).into(),
                    (
                        json!({ "format": "date" }),
                        json!({ "format": "time" }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/format".to_string(),
                            value: json!("time"),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "contentMediaType",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "contentMediaType": "application/x.dash.dpp.identifier" }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/contentMediaType".to_string(),
                            value: json!("application/x.dash.dpp.identifier"),
                        })),
                    )
                        .into(),
                    (
                        json!({ "contentMediaType": "application/x.dash.dpp.identifier" }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/contentMediaType".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "contentMediaType": "application/x.dash.dpp.identifier" }),
                        json!({ "contentMediaType": "application/unknown" }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/contentMediaType".to_string(),
                            value: json!("application/unknown"),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "byteArray",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "byteArray": true }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/byteArray".to_string(),
                            value: json!(true),
                        })),
                    )
                        .into(),
                    (
                        json!({ "byteArray": true }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/byteArray".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "byteArray": true }),
                        json!({ "byteArray": false }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/byteArray".to_string(),
                            value: json!(false),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "prefixItems",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: None,
                subschema_levels_depth: Some(2),
                inner: Some(Box::new(CompatibilityRules {
                    allow_addition: true,
                    allow_removal: false,
                    allow_replacement_callback: None,
                    subschema_levels_depth: None,
                    inner: None,
                    #[cfg(any(test, feature = "examples"))]
                    examples: vec![
                        (
                            json!({ "prefixItems": [{ "type": "string" }] }),
                            json!({ "prefixItems": [{ "type": "string" }, { "type": "number"}] }),
                            None,
                        )
                            .into(),
                        (
                            json!({ "prefixItems": [{ "type": "string" }, { "type": "number"}] }),
                            json!({ "prefixItems": [{ "type": "string" }] }),
                            Some(JsonSchemaChange::Remove(RemoveOperation {
                                path: "/prefixItems/1".to_string(),
                            })),
                        )
                            .into(),
                    ],
                })),
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "prefixItems": [{ "type": "string" }] }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/prefixItems".to_string(),
                            value: json!([{ "type": "string" }]),
                        })),
                    )
                        .into(),
                    (
                        json!({ "prefixItems": [{ "type": "string" }] }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/prefixItems".to_string(),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "items",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: Some(1),
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "items": { "type": "string" } }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/items".to_string(),
                            value: json!({ "type": "string" }),
                        })),
                    )
                        .into(),
                    (
                        json!({ "items": { "type": "string" } }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/items".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "items": { "type": "string" } }),
                        json!({ "items": false }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/items".to_string(),
                            value: json!(false),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "position",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "position": 0 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/position".to_string(),
                            value: json!(0),
                        })),
                    )
                        .into(),
                    (
                        json!({ "position": 0 }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/position".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "position": 0 }),
                        json!({ "position": 1 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/position".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "$defs",
            CompatibilityRules {
                allow_addition: true,
                allow_removal: false,
                allow_replacement_callback: None,
                subschema_levels_depth: Some(2),
                inner: Some(Box::new(CompatibilityRules {
                    allow_addition: true,
                    allow_removal: false,
                    allow_replacement_callback: None,
                    subschema_levels_depth: None,
                    inner: None,
                    #[cfg(any(test, feature = "examples"))]
                    examples: vec![
                        (
                            json!({ "$defs": {"definition1": {}} }),
                            json!({ "$defs": {"definition1": {}, "definition2": {}} }),
                            None,
                        )
                            .into(),
                        (
                            json!({ "$defs": {"definition1": {}, "definition2": {}} }),
                            json!({ "$defs": {"definition1": {}} }),
                            Some(JsonSchemaChange::Remove(RemoveOperation {
                                path: "/$defs/definition2".to_string(),
                            })),
                        )
                            .into(),
                    ],
                })),
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (json!({}), json!({ "$defs": {"definition": {}} }), None).into(),
                    (
                        json!({ "$defs": {"definition": {}} }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/$defs".to_string(),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "documentsMutable",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "documentsMutable": false }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/documentsMutable".to_string(),
                            value: json!(false),
                        })),
                    )
                        .into(),
                    (
                        json!({ "documentsMutable": false }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/documentsMutable".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "documentsMutable": false }),
                        json!({ "documentsMutable": true }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/documentsMutable".to_string(),
                            value: json!(true),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "documentsKeepHistory",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "documentsKeepHistory": false }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/documentsKeepHistory".to_string(),
                            value: json!(false),
                        })),
                    )
                        .into(),
                    (
                        json!({ "documentsKeepHistory": false }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/documentsKeepHistory".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "documentsKeepHistory": false }),
                        json!({ "documentsKeepHistory": true }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/documentsKeepHistory".to_string(),
                            value: json!(true),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "canBeDeleted",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "canBeDeleted": false }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/canBeDeleted".to_string(),
                            value: json!(false),
                        })),
                    )
                        .into(),
                    (
                        json!({ "canBeDeleted": false }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/canBeDeleted".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "canBeDeleted": false }),
                        json!({ "canBeDeleted": true }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/canBeDeleted".to_string(),
                            value: json!(true),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "transferable",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "transferable": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/transferable".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (
                        json!({ "transferable": 1 }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/transferable".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "transferable": 0 }),
                        json!({ "transferable": 1 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/transferable".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "tradeMode",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "tradeMode": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/tradeMode".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (
                        json!({ "tradeMode": 1 }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/tradeMode".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "tradeMode": 0 }),
                        json!({ "tradeMode": 1 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/tradeMode".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "creationRestrictionMode",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "creationRestrictionMode": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/creationRestrictionMode".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (
                        json!({ "creationRestrictionMode": 1 }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/creationRestrictionMode".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "creationRestrictionMode": 0 }),
                        json!({ "creationRestrictionMode": 1 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/creationRestrictionMode".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "requiresIdentityEncryptionBoundedKey",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "requiresIdentityEncryptionBoundedKey": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/requiresIdentityEncryptionBoundedKey".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (
                        json!({ "requiresIdentityEncryptionBoundedKey": 1 }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/requiresIdentityEncryptionBoundedKey".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "requiresIdentityEncryptionBoundedKey": 0 }),
                        json!({ "requiresIdentityEncryptionBoundedKey": 1 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/requiresIdentityEncryptionBoundedKey".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "requiresIdentityDecryptionBoundedKey",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "requiresIdentityDecryptionBoundedKey": 1 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/requiresIdentityDecryptionBoundedKey".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                    (
                        json!({ "requiresIdentityDecryptionBoundedKey": 1 }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/requiresIdentityDecryptionBoundedKey".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "requiresIdentityDecryptionBoundedKey": 0 }),
                        json!({ "requiresIdentityDecryptionBoundedKey": 1 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/requiresIdentityDecryptionBoundedKey".to_string(),
                            value: json!(1),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "signatureSecurityLevelRequirement",
            CompatibilityRules {
                allow_addition: false,
                allow_removal: false,
                allow_replacement_callback: FALSE_CALLBACK.clone(),
                subschema_levels_depth: None,
                inner: None,
                #[cfg(any(test, feature = "examples"))]
                examples: vec![
                    (
                        json!({}),
                        json!({ "signatureSecurityLevelRequirement": false }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/signatureSecurityLevelRequirement".to_string(),
                            value: json!(false),
                        })),
                    )
                        .into(),
                    (
                        json!({ "signatureSecurityLevelRequirement": false }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/signatureSecurityLevelRequirement".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "signatureSecurityLevelRequirement": false }),
                        json!({ "signatureSecurityLevelRequirement": true }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/signatureSecurityLevelRequirement".to_string(),
                            value: json!(true),
                        })),
                    )
                        .into(),
                ],
            },
        ),
    ])
});
