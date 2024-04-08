use once_cell::sync::Lazy;
#[cfg(test)]
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::HashMap;

pub static KEYWORDS: Lazy<HashMap<&'static str, KeywordRule>> = Lazy::new(|| {
    HashMap::from_iter(vec![
        (
            "$id",
            KeywordRule {
                allow_adding: true,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new("foo", "bar", None),
            },
        ),
        (
            "$ref",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new("foo", "bar", None),
            },
        ),
        (
            "$comment",
            KeywordRule {
                allow_adding: true,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new("foo", "bar", None),
            },
        ),
        (
            "description",
            KeywordRule {
                allow_adding: true,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new("foo", "bar", None),
            },
        ),
        (
            "examples",
            KeywordRule {
                allow_adding: true,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(
                    JsonValue::Array(vec![JsonValue::from("foo")]),
                    JsonValue::Array(vec![JsonValue::from("foo")]),
                    Some(JsonValue::Array(vec![
                        JsonValue::from("foo"),
                        JsonValue::from("boo"),
                    ])),
                ),
            },
        ),
        (
            "multiple_of",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(123, 456, None),
            },
        ),
        (
            "maximum",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(123, 122, Some(124)),
            },
        ),
        (
            "exclusiveMaximum",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(123, 122, Some(124)),
            },
        ),
        (
            "minimum",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(123, 124, Some(122)),
            },
        ),
        (
            "exclusiveMinimum",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(123, 124, Some(122)),
            },
        ),
        (
            "maxLength",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(123, 122, Some(124)),
            },
        ),
        (
            "minLength",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(123, 124, Some(122)),
            },
        ),
        (
            "pattern",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new("[a-z]", "[0-9]", None),
            },
        ),
        (
            "maxItems",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(123, 122, Some(124)),
            },
        ),
        (
            "minItems",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(123, 124, Some(122)),
            },
        ),
        (
            "uniqueItems",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(true, false, Some(true)),
            },
        ),
        (
            "contains",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(
                    JsonValue::Object(JsonMap::from_iter(vec![(
                        String::from("type"),
                        JsonValue::from("string"),
                    )])),
                    JsonValue::Object(JsonMap::from_iter(vec![(
                        String::from("type"),
                        JsonValue::from("number"),
                    )])),
                    None,
                ),
            },
        ),
        (
            "required",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(
                    JsonValue::Array(vec![JsonValue::from("property1")]),
                    JsonValue::Array(vec![
                        JsonValue::from("property1"),
                        JsonValue::from("property2"),
                    ]),
                    Some(JsonValue::Array(Vec::new())),
                ),
            },
        ),
        (
            "properties",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(
                    JsonValue::Object(JsonMap::from_iter(vec![(
                        String::from("property1"),
                        JsonValue::Object(JsonMap::new()),
                    )])),
                    JsonValue::Object(JsonMap::from_iter(vec![(
                        String::from("property2"),
                        JsonValue::Object(JsonMap::new()),
                    )])),
                    None,
                ),
            },
        ),
        (
            "additionalProperties",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(true, false, None),
            },
        ),
        (
            "dependentSchemas",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(
                    JsonValue::Object(JsonMap::from_iter(vec![(
                        String::from("property1"),
                        JsonValue::Object(JsonMap::new()),
                    )])),
                    JsonValue::Object(JsonMap::from_iter(vec![(
                        String::from("property2"),
                        JsonValue::Object(JsonMap::new()),
                    )])),
                    None,
                ),
            },
        ),
        (
            "dependentRequired",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(
                    JsonValue::Object(JsonMap::from_iter(vec![(
                        String::from("property1"),
                        JsonValue::Array(Vec::new()),
                    )])),
                    JsonValue::Object(JsonMap::from_iter(vec![(
                        String::from("property2"),
                        JsonValue::Array(Vec::new()),
                    )])),
                    None,
                ),
            },
        ),
        (
            "const",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new("foo", "boo", None),
            },
        ),
        (
            "enum",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: true,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(
                    JsonValue::Array(vec![JsonValue::from(1), JsonValue::from(2)]),
                    JsonValue::Array(vec![JsonValue::from(1)]),
                    Some(JsonValue::Array(vec![
                        JsonValue::from(1),
                        JsonValue::from(2),
                        JsonValue::from(3),
                    ])),
                ),
            },
        ),
        (
            "type",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new("string", "object", None),
            },
        ),
        (
            "format",
            KeywordRule {
                allow_adding: false,
                allow_removing: true,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new("date", "time", None),
            },
        ),
        (
            "contentMediaType",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(
                    "application/x.dash.dpp.identifier",
                    "application/unknown",
                    None,
                ),
            },
        ),
        (
            "byteArray",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(true, false, None),
            },
        ),
        (
            "prefixItems",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(
                    JsonValue::Array(vec![]),
                    JsonValue::Array(vec![JsonValue::Object(JsonMap::new())]),
                    None,
                ),
            },
        ),
        (
            "items",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(
                    JsonValue::Object(JsonMap::from_iter(vec![(
                        String::from("type"),
                        JsonValue::from("string"),
                    )])),
                    JsonValue::Object(JsonMap::from_iter(vec![(
                        String::from("type"),
                        JsonValue::from("object"),
                    )])),
                    None,
                ),
            },
        ),
        (
            "position",
            KeywordRule {
                allow_adding: false,
                allow_removing: false,
                allow_replacing: false,
                #[cfg(test)]
                examples: KeywordRuleExamples::new(0, 1, None),
            },
        ),
    ])
});

pub struct KeywordRule {
    pub allow_adding: bool,
    pub allow_removing: bool,
    pub allow_replacing: bool,
    #[cfg(test)]
    pub examples: KeywordRuleExamples,
}

#[cfg(test)]
pub struct KeywordRuleExamples {
    previous_value: JsonValue,
    new_invalid_value: JsonValue,
    new_valid_value: Option<JsonValue>,
}

#[cfg(test)]
impl KeywordRuleExamples {
    fn new<V>(previous_value: V, new_invalid_value: V, new_valid_value: Option<V>) -> Self
    where
        V: Into<JsonValue>,
    {
        Self {
            previous_value: previous_value.into(),
            new_invalid_value: new_invalid_value.into(),
            new_valid_value: new_valid_value.map(Into::into),
        }
    }
}
