#[cfg(test)]
use crate::change::JsonSchemaChange;
use crate::keyword::keyword_rule::KeywordRule;
use crate::keyword::ReplaceCallback;
#[cfg(test)]
use json_patch::{AddOperation, RemoveOperation, ReplaceOperation};
use once_cell::sync::Lazy;
#[cfg(test)]
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

static FALSE_CALLBACK: Lazy<ReplaceCallback> = Lazy::new(|| Some(Arc::new(|_, _| false)));
static TRUE_CALLBACK: Lazy<ReplaceCallback> = Lazy::new(|| Some(Arc::new(|_, _| true)));

pub(crate) static KEYWORD_RULES: Lazy<HashMap<&'static str, KeywordRule>> = Lazy::new(|| {
    HashMap::from_iter([
        (
            "$id",
            KeywordRule {
                allow_adding: true,
                allow_removing: false,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: true,
                allow_removing: true,
                allow_replacing: TRUE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: true,
                allow_removing: true,
                allow_replacing: TRUE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: true,
                allow_removing: true,
                allow_replacing: None,
                levels_to_subschema: None,
                inner: Some(Box::new(KeywordRule {
                    allow_adding: true,
                    allow_removing: true,
                    allow_replacing: TRUE_CALLBACK.clone(),
                    levels_to_subschema: None,
                    inner: None,
                    #[cfg(test)]
                    examples: vec![
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
                })),
                #[cfg(test)]
                examples: vec![
                    (json!({}), json!({ "examples": ["foo"] }), None).into(),
                    (json!({ "examples": ["foo"] }), json!({}), None).into(),
                ],
            },
        ),
        (
            "multiple_of",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
                examples: vec![
                    (
                        json!({}),
                        json!({ "multiple_of": 1.0 }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/multiple_of".to_string(),
                            value: json!(1.0),
                        })),
                    )
                        .into(),
                    (
                        json!({ "multiple_of": 1.0 }),
                        json!({}),
                        Some(JsonSchemaChange::Remove(RemoveOperation {
                            path: "/multiple_of".to_string(),
                        })),
                    )
                        .into(),
                    (
                        json!({ "multiple_of": 1.0 }),
                        json!({ "multiple_of": 2.0 }),
                        Some(JsonSchemaChange::Replace(ReplaceOperation {
                            path: "/multiple_of".to_string(),
                            value: json!(2.0),
                        })),
                    )
                        .into(),
                ],
            },
        ),
        (
            "maximum",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: Some(Arc::new(|previous, new| previous.as_f64() < new.as_f64())),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: Some(Arc::new(|previous, new| previous.as_f64() < new.as_f64())),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: Some(Arc::new(|previous, new| previous.as_f64() > new.as_f64())),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: Some(Arc::new(|previous, new| previous.as_f64() > new.as_f64())),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: Some(Arc::new(|previous, new| previous.as_u64() < new.as_u64())),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: Some(Arc::new(|previous, new| previous.as_u64() > new.as_u64())),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: Some(Arc::new(|previous, new| previous.as_u64() < new.as_u64())),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: Some(Arc::new(|previous, new| previous.as_u64() > new.as_u64())),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: Some(Arc::new(|previous, new| {
                    previous.as_bool().expect("value must be boolean")
                        && !new.as_bool().expect("value must be boolean")
                })),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            "contains",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: None,
                levels_to_subschema: Some(1),
                inner: None,
                #[cfg(test)]
                examples: vec![
                    (
                        json!({}),
                        json!({ "contains": { "type": "string" } }),
                        Some(JsonSchemaChange::Add(AddOperation {
                            path: "/contains".to_string(),
                            value: json!({ "type": "string" }),
                        })),
                    )
                        .into(),
                    (json!({ "contains": { "type": "string" } }), json!({}), None).into(),
                ],
            },
        ),
        (
            "required",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: None,
                levels_to_subschema: None,
                inner: Some(Box::new(KeywordRule {
                    allow_adding: false,
                    allow_removing: true,
                    allow_replacing: FALSE_CALLBACK.clone(),
                    levels_to_subschema: None,
                    inner: None,
                    #[cfg(test)]
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
                            json!({ "required": ["foo"] }),
                            json!({ "required": ["bar"] }),
                            Some(JsonSchemaChange::Replace(ReplaceOperation {
                                path: "/required/0".to_string(),
                                value: json!("bar"),
                            })),
                        )
                            .into(),
                    ],
                })),
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: None,
                levels_to_subschema: Some(2),
                inner: Some(Box::new(KeywordRule {
                    allow_adding: true,
                    allow_removing: false,
                    allow_replacing: None,
                    levels_to_subschema: None,
                    inner: None,
                    #[cfg(test)]
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
                        // TODO: We should handle property names that matches keywords
                        // (
                        //     json!({ "properties": {"foo": {}} }),
                        //     json!({ "properties": {"foo": {}, "type": {}} }),
                        //     None,
                        // )
                        //     .into(),
                    ],
                })),
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: None,
                levels_to_subschema: None,
                inner: None,
                // TODO: The same problems as property name that matches keywords
                // inner: Some(Box::new(KeywordRule {
                //     allow_adding: false,
                //     allow_removing: true,
                //     allow_replacing: FALSE_CALLBACK.clone(),
                //     inner: None,
                //     #[cfg(test)]
                //     examples: vec![
                //         (
                //             json!({ "dependentRequired": {"foo": ["bar"]} }),
                //             json!({ "dependentRequired": {"foo": ["bar", "baz"]} }),
                //             Some(JsonSchemaChange::Add(AddOperation {
                //                 path: "/dependentRequired/foo/1".to_string(),
                //                 value: json!("baz"),
                //             })),
                //         )
                //             .into(),
                //         (
                //             json!({ "dependentRequired": {"foo": ["bar", "baz"]} }),
                //             json!({ "dependentRequired": {"foo": ["bar"]} }),
                //             None,
                //         )
                //             .into(),
                //         (
                //             json!({ "dependentRequired": {"foo": ["bar"]} }),
                //             json!({ "dependentRequired": {"foo": ["baz"]} }),
                //             Some(JsonSchemaChange::Replace(ReplaceOperation {
                //                 path: "/dependentRequired/foo/0".to_string(),
                //                 value: json!("baz"),
                //             })),
                //         )
                //             .into(),
                //     ],
                // })),
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: Some(1),
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: None,
                levels_to_subschema: Some(2),
                inner: Some(Box::new(KeywordRule {
                    allow_adding: true,
                    allow_removing: false,
                    allow_replacing: FALSE_CALLBACK.clone(),
                    levels_to_subschema: None,
                    inner: None,
                    #[cfg(test)]
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
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: None,
                levels_to_subschema: Some(2),
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: Some(1),
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: FALSE_CALLBACK.clone(),
                levels_to_subschema: None,
                inner: None,
                #[cfg(test)]
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
            KeywordRule {
                allow_adding: true,
                allow_removing: false,
                allow_replacing: None,
                levels_to_subschema: Some(2),
                inner: Some(Box::new(KeywordRule {
                    allow_adding: true,
                    allow_removing: false,
                    allow_replacing: None,
                    levels_to_subschema: None,
                    inner: None,
                    #[cfg(test)]
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
                #[cfg(test)]
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
    ])
});
// TODO: Add inners
//
// pub(crate) static KEYWORD_INNER_RULES: Lazy<HashMap<&'static str, KeywordRule>> = Lazy::new(|| {
//     HashMap::from_iter([
//         // (
//         //     "prefixItems",
//         //     KeywordRule {
//         //         allow_adding: false,
//         //         allow_removing: false,
//         //         allow_replacing: *FALSE_CALLBACK,
//         //         #[cfg(test)]
//         //         examples: KeywordRuleExamples::new(
//         //             Value::Array(vec![]),
//         //             Value::Array(vec![Value::Object(ValueMap::new())]),
//         //             None,
//         //         ),
//         //     },
//         // ),
//         // (
//         //     "items",
//         //     KeywordRule {
//         //         allow_adding: false,
//         //         allow_removing: false,
//         //         allow_replacing: *FALSE_CALLBACK,
//         //         #[cfg(test)]
//         //         examples: KeywordRuleExamples::new(
//         //             Value::Object(ValueMap::from_iter([(
//         //                 String::from("type"),
//         //                 Value::from("string"),
//         //             )])),
//         //             Value::Object(ValueMap::from_iter([(
//         //                 String::from("type"),
//         //                 Value::from("object"),
//         //             )])),
//         //             None,
//         //         ),
//         //     },
//         // ),
//     ])
// });
