//! The schema checks parser generation 4 runs at registration in place of the document
//! meta-schema (`schema/meta_schemas/document/v3/document-meta.json`), which the
//! `jsonschema` crate checked the enriched schema against through protocol version 14.
//!
//! [`check_document_type_schema`] repeats that meta-schema's rules, read from the JSON the
//! crate checked, so a repeated key reads as its last entry and every value as its JSON
//! form, with the crate's readings of an integer (a float with no fractional part is one),
//! a number bound (exact across integer and float readings), a `const` number (within
//! `f64::EPSILON`) and an array of unique items. A schema breaking one is refused with the
//! [`JsonSchemaError`] the meta-schema check returned, so the error code stays.
//!
//! Generation 4 differs from the meta-schema in one rule: an index property name is at most
//! 255 bytes, the longest key GroveDB stores, where the meta-schema took 256 characters.
//!
//! [`check_validation_schema`] then requires the type to have a typed validation schema,
//! which documents of every type registered from protocol version 15 are checked against
//! natively.
//!
//! [`JsonSchemaError`]: crate::consensus::basic::json_schema_error::JsonSchemaError

use crate::consensus::basic::json_schema_error::JsonSchemaError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::data_contract::document_type::schema::validation_schema::{
    is_unique, validator_regex_within, ValidationSchema,
};
use crate::data_contract::document_type::DocumentType;
use crate::ProtocolError;
use num_cmp::NumCmp;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use serde_json::{json, Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use std::collections::BTreeMap;

mod document_type;
mod properties;

/// Checks `root`, a document type's schema with the contract's `$defs` and the base schema
/// added and its property type shorthands expanded, against the rules of the document
/// meta-schema, as parser generation 4 does under full validation.
pub(super) fn check_document_type_schema(root: &Value) -> Result<(), ProtocolError> {
    let root = root.try_to_validating_json().map_err(|e| {
        ProtocolError::ConsensusError(
            ConsensusError::BasicError(BasicError::ValueError(e.into())).into(),
        )
    })?;
    Checker::default().root(&root)
}

/// Requires the type whose expanded schema is `schema`, with the contract's expanded
/// `$defs`, to have a typed validation schema, refusing one it does not represent.
pub(super) fn check_validation_schema(
    schema: &Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    let root = DocumentType::enrich_with_base_schema(
        schema.clone(),
        schema_defs.map(|defs| Value::from(defs.clone())),
        platform_version,
    )?;
    ValidationSchema::try_from_root(&root).map_err(consensus_or_protocol_data_contract_error)?;
    Ok(())
}

/// A check of one value of a document schema, knowing where the value is.
#[derive(Default)]
struct Checker<'a> {
    /// The check of the value holding this one, and this value's key or index in it.
    parent: Option<(&'a Checker<'a>, String)>,
}

type Checked = Result<(), ProtocolError>;

impl Checker<'_> {
    /// Runs `check` on the member `segment` of the value in hand.
    fn at<T>(
        &self,
        segment: impl ToString,
        check: impl FnOnce(&Checker<'_>) -> Result<T, ProtocolError>,
    ) -> Result<T, ProtocolError> {
        check(&Checker {
            parent: Some((self, segment.to_string())),
        })
    }

    /// The JSON pointer to the value in hand.
    fn instance_path(&self) -> String {
        match &self.parent {
            None => String::new(),
            Some((parent, segment)) => format!(
                "{}/{}",
                parent.instance_path(),
                segment.replace('~', "~0").replace('/', "~1")
            ),
        }
    }

    /// The error of the value in hand failing `keyword`.
    fn fail(&self, keyword: &str, message: impl Into<String>) -> ProtocolError {
        self.fail_with(keyword, message, json!({}), "")
    }

    fn fail_with(
        &self,
        keyword: &str,
        message: impl Into<String>,
        params: JsonValue,
        property_name: &str,
    ) -> ProtocolError {
        let instance_path = self.instance_path();
        ProtocolError::ConsensusError(Box::new(ConsensusError::BasicError(
            BasicError::JsonSchemaError(JsonSchemaError::new(
                message.into(),
                keyword.to_string(),
                instance_path,
                String::new(),
                params.into(),
                property_name.to_string(),
            )),
        )))
    }

    // ---- primitives, as the meta-schema check read them ----

    fn object<'v>(
        &self,
        value: &'v JsonValue,
    ) -> Result<&'v JsonMap<String, JsonValue>, ProtocolError> {
        value
            .as_object()
            .ok_or_else(|| self.fail("type", format!("{value} is not of type \"object\"")))
    }

    fn array<'v>(&self, value: &'v JsonValue) -> Result<&'v Vec<JsonValue>, ProtocolError> {
        value
            .as_array()
            .ok_or_else(|| self.fail("type", format!("{value} is not of type \"array\"")))
    }

    fn string<'v>(&self, value: &'v JsonValue) -> Result<&'v str, ProtocolError> {
        value
            .as_str()
            .ok_or_else(|| self.fail("type", format!("{value} is not of type \"string\"")))
    }

    fn boolean(&self, value: &JsonValue) -> Checked {
        if value.is_boolean() {
            Ok(())
        } else {
            Err(self.fail("type", format!("{value} is not of type \"boolean\"")))
        }
    }

    fn number(&self, value: &JsonValue) -> Checked {
        if value.is_number() {
            Ok(())
        } else {
            Err(self.fail("type", format!("{value} is not of type \"number\"")))
        }
    }

    /// An integer, as the meta-schema check read one, from `minimum` to `maximum`.
    fn integer(&self, value: &JsonValue, minimum: Option<i64>, maximum: Option<i64>) -> Checked {
        let Some(number) = value.as_number().filter(|number| is_integer(number)) else {
            return Err(self.fail("type", format!("{value} is not of type \"integer\"")));
        };
        if let Some(minimum) = minimum {
            if number_below(number, minimum) {
                return Err(self.fail_with(
                    "minimum",
                    format!("{value} is less than the minimum of {minimum}"),
                    json!({ "minimum": minimum }),
                    "",
                ));
            }
        }
        if let Some(maximum) = maximum {
            if number_above(number, maximum) {
                return Err(self.fail_with(
                    "maximum",
                    format!("{value} is greater than the maximum of {maximum}"),
                    json!({ "maximum": maximum }),
                    "",
                ));
            }
        }
        Ok(())
    }

    fn non_negative_integer(&self, value: &JsonValue) -> Checked {
        self.integer(value, Some(0), None)
    }

    /// A string of `minimum` to `maximum` characters.
    fn string_of_length<'v>(
        &self,
        value: &'v JsonValue,
        minimum: usize,
        maximum: Option<usize>,
    ) -> Result<&'v str, ProtocolError> {
        let text = self.string(value)?;
        let length = text.chars().count();
        if length < minimum {
            return Err(self.fail_with(
                "minLength",
                format!("{value} is shorter than {minimum} characters"),
                json!({ "minLength": minimum }),
                "",
            ));
        }
        if let Some(maximum) = maximum {
            if length > maximum {
                return Err(self.fail_with(
                    "maxLength",
                    format!("{value} is longer than {maximum} characters"),
                    json!({ "maxLength": maximum }),
                    "",
                ));
            }
        }
        Ok(text)
    }

    /// One of `allowed`, compared as the meta-schema check compared an `enum` member.
    fn one_of(&self, value: &JsonValue, allowed: &[JsonValue]) -> Checked {
        if allowed.iter().any(|member| equal_json(value, member)) {
            Ok(())
        } else {
            Err(self.fail_with(
                "enum",
                format!(
                    "{value} is not one of {}",
                    JsonValue::from(allowed.to_vec())
                ),
                json!({ "enum": allowed }),
                "",
            ))
        }
    }

    fn one_of_strings(&self, value: &JsonValue, allowed: &[&str]) -> Checked {
        self.one_of(
            value,
            &allowed.iter().map(|text| json!(text)).collect::<Vec<_>>(),
        )
    }

    fn one_of_integers(&self, value: &JsonValue, allowed: &[u64]) -> Checked {
        self.one_of(
            value,
            &allowed
                .iter()
                .map(|number| json!(number))
                .collect::<Vec<_>>(),
        )
    }

    /// Equal to `expected`, as the meta-schema check compared a `const`: a number within
    /// `f64::EPSILON`.
    fn constant(&self, value: &JsonValue, expected: &JsonValue) -> Checked {
        let equal = match (value, expected) {
            (_, JsonValue::Number(expected)) => value.as_f64().is_some_and(|value| {
                (expected.as_f64().unwrap_or(f64::NAN) - value).abs() < f64::EPSILON
            }),
            _ => equal_json(value, expected),
        };
        if equal {
            Ok(())
        } else {
            Err(self.fail_with(
                "const",
                format!("{expected} was expected"),
                json!({ "allowedValue": expected }),
                "",
            ))
        }
    }

    /// No key of `map` outside `allowed`.
    fn closed(&self, map: &JsonMap<String, JsonValue>, allowed: &[&str]) -> Checked {
        let unexpected: Vec<&String> = map
            .keys()
            .filter(|key| !allowed.contains(&key.as_str()))
            .collect();
        match unexpected.first() {
            None => Ok(()),
            Some(first) => Err(self.fail_with(
                "additionalProperties",
                format!("additional properties are not allowed ({unexpected:?} were unexpected)"),
                json!({ "additionalProperties": unexpected }),
                first,
            )),
        }
    }

    fn required(&self, map: &JsonMap<String, JsonValue>, names: &[&str]) -> Checked {
        match names.iter().find(|name| !map.contains_key(**name)) {
            None => Ok(()),
            Some(missing) => Err(self.fail_with(
                "required",
                format!("\"{missing}\" is a required property"),
                json!({ "missingProperty": missing }),
                missing,
            )),
        }
    }

    /// `name` absent from `map`, as a `false` subschema for it requires.
    fn absent(&self, map: &JsonMap<String, JsonValue>, name: &str) -> Checked {
        if map.contains_key(name) {
            self.at(name, |c| {
                Err(c.fail("falseSchema", format!("\"{name}\" is not allowed here")))
            })
        } else {
            Ok(())
        }
    }

    /// `check`, as a branch of the meta-schema's `keyword` (`anyOf` or `oneOf`): a value it
    /// refuses is refused here as matching no branch, where the meta-schema check reported it.
    fn within(
        &self,
        keyword: &str,
        value: &JsonValue,
        check: impl FnOnce(&Self) -> Checked,
    ) -> Checked {
        check(self).map_err(|_| {
            self.fail(
                keyword,
                format!("{value} is not valid under any of the given schemas"),
            )
        })
    }

    fn property_count(
        &self,
        map: &JsonMap<String, JsonValue>,
        minimum: Option<usize>,
        maximum: Option<usize>,
    ) -> Checked {
        if let Some(minimum) = minimum {
            if map.len() < minimum {
                return Err(self.fail_with(
                    "minProperties",
                    format!("an object with fewer than {minimum} properties is not allowed"),
                    json!({ "minProperties": minimum }),
                    "",
                ));
            }
        }
        if let Some(maximum) = maximum {
            if map.len() > maximum {
                return Err(self.fail_with(
                    "maxProperties",
                    format!("an object with more than {maximum} properties is not allowed"),
                    json!({ "maxProperties": maximum }),
                    "",
                ));
            }
        }
        Ok(())
    }

    fn item_count(
        &self,
        items: &[JsonValue],
        minimum: Option<usize>,
        maximum: Option<usize>,
    ) -> Checked {
        if let Some(minimum) = minimum {
            if items.len() < minimum {
                return Err(self.fail_with(
                    "minItems",
                    format!("an array with fewer than {minimum} items is not allowed"),
                    json!({ "minItems": minimum }),
                    "",
                ));
            }
        }
        if let Some(maximum) = maximum {
            if items.len() > maximum {
                return Err(self.fail_with(
                    "maxItems",
                    format!("an array with more than {maximum} items is not allowed"),
                    json!({ "maxItems": maximum }),
                    "",
                ));
            }
        }
        Ok(())
    }

    fn unique(&self, items: &[JsonValue]) -> Checked {
        if is_unique(items) {
            Ok(())
        } else {
            Err(self.fail("uniqueItems", "the array has non-unique elements"))
        }
    }

    /// A name of 1 to 64 ASCII letters, digits and underscores.
    fn word(&self, value: &JsonValue) -> Checked {
        let text = self.string(value)?;
        if is_word(text) {
            Ok(())
        } else {
            Err(self.fail_with(
                "pattern",
                format!("{value} does not match \"^[a-zA-Z0-9_]{{1,64}}$\""),
                json!({ "pattern": "^[a-zA-Z0-9_]{1,64}$" }),
                "",
            ))
        }
    }

    /// An array of strings, no two alike.
    fn string_array(&self, value: &JsonValue) -> Checked {
        let items = self.array(value)?;
        for (index, item) in items.iter().enumerate() {
            self.at(index, |c| c.string(item).map(drop))?;
        }
        self.unique(items)
    }

    /// `check` on the member `name` of `map`, when there is one.
    fn member(
        &self,
        map: &JsonMap<String, JsonValue>,
        name: &str,
        check: impl FnOnce(&Checker<'_>, &JsonValue) -> Checked,
    ) -> Checked {
        match map.get(name) {
            Some(value) => self.at(name, |c| check(c, value)),
            None => Ok(()),
        }
    }

    fn member_constant(
        &self,
        map: &JsonMap<String, JsonValue>,
        name: &str,
        expected: JsonValue,
    ) -> Checked {
        self.member(map, name, |c, value| c.constant(value, &expected))
    }
}

/// A name of 1 to 64 ASCII letters, digits and underscores.
fn is_word(text: &str) -> bool {
    (1..=64).contains(&text.len()) && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether `number` is an integer as the meta-schema check read one: unsigned, signed, or
/// a float with no fractional part.
fn is_integer(number: &JsonNumber) -> bool {
    number.is_u64() || number.is_i64() || number.as_f64().is_some_and(|number| number.fract() == 0.)
}

/// Whether `number` is below `limit`, exactly across its integer and float readings.
fn number_below(number: &JsonNumber, limit: i64) -> bool {
    if let Some(number) = number.as_u64() {
        NumCmp::num_lt(number, limit)
    } else if let Some(number) = number.as_i64() {
        NumCmp::num_lt(number, limit)
    } else {
        NumCmp::num_lt(number.as_f64().unwrap_or(f64::NAN), limit)
    }
}

/// Whether `number` is above `limit`, exactly across its integer and float readings.
fn number_above(number: &JsonNumber, limit: i64) -> bool {
    if let Some(number) = number.as_u64() {
        NumCmp::num_gt(number, limit)
    } else if let Some(number) = number.as_i64() {
        NumCmp::num_gt(number, limit)
    } else {
        NumCmp::num_gt(number.as_f64().unwrap_or(f64::NAN), limit)
    }
}

/// Whether two JSON values are equal as the meta-schema check compared an `enum` member:
/// numbers by value whatever their reading, objects entry by entry in written order.
fn equal_json(left: &JsonValue, right: &JsonValue) -> bool {
    match (left, right) {
        (JsonValue::Number(left), JsonValue::Number(right)) => {
            match (left.as_u64(), left.as_i64(), right.as_u64(), right.as_i64()) {
                (Some(left), _, Some(right), _) => left == right,
                (_, Some(left), _, Some(right)) => left == right,
                _ => match (left.as_f64(), right.as_f64()) {
                    (Some(left), Some(right)) => {
                        if let Some(right_integer) = right.fract().eq(&0.).then_some(right) {
                            left == right_integer
                        } else {
                            left == right
                        }
                    }
                    _ => false,
                },
            }
        }
        (JsonValue::Array(left), JsonValue::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(l, r)| equal_json(l, r))
        }
        (JsonValue::Object(left), JsonValue::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|((lk, lv), (rk, rv))| lk == rk && equal_json(lv, rv))
        }
        (left, right) => left == right,
    }
}
