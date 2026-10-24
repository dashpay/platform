//! The native validator of a document type's schema: it checks a document value against
//! the [`ValidationSchema`] as the JSON Schema validator the document validator compiles
//! checks it against the schema the tree was built from.
//!
//! The value checked is the JSON the document validator converts a document's properties
//! to. Each keyword repeats the pinned fork's check, with the crates the fork uses where it
//! uses one: bounds compare exactly across the unsigned, signed and float readings of two
//! numbers (`num-cmp`); `multipleOf` divides in f64 and, when the quotient overflows,
//! decides on `fraction`'s reading of both numbers; `enum`, `const` and `uniqueItems`
//! compare as their [`SchemaNode`] fields say; a string's length counts its characters; a
//! `pattern` is searched for anywhere in the string; and each format reads a string as the
//! fork reads it (the `time` crate for dates, `url` for URIs, the standard library for IP
//! addresses). `maxBytes` and `encryptedFor` are not checked here: document validation
//! checks them in steps of their own, on the properties the document type parser builds.
//!
//! The walk stops at the first violation it meets, the one consensus reports, and meets
//! them in a fixed order, not in the order the schema writes its keywords, where the JSON
//! Schema validator lists them. A node's `$ref` is followed first, then `type`, `enum` and `const`, then the keywords of the value's kind:
//! an object's declared properties in declared order, `additionalProperties`, `required`,
//! `minProperties`, `maxProperties` and `dependentRequired` by name; an array's `items`,
//! `contains`, `minItems`, `maxItems` and `uniqueItems`; a string's `minLength`,
//! `maxLength` and `pattern`; a number's bounds and `multipleOf`; then `format` and
//! `byteArray`. A value breaking one keyword gets the violation the JSON Schema validator
//! reports, and every violation is a `JsonSchemaError`, so the verdict and the error code
//! are the same; a value breaking several can get another violation first.

use super::{
    structure_error, validator_regex, ArrayConstraints, JsonType, JsonTypes, NumberConstraints,
    ObjectConstraints, SchemaNode, StringConstraints, ValidationSchema, ValidatorFormat,
};
use crate::consensus::basic::json_schema_error::JsonSchemaError;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use fraction::{BigFraction, BigUint};
use num_cmp::NumCmp;
use once_cell::sync::Lazy;
use platform_value::Value;
use regex::Regex;
use serde_json::{json, Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::hash::{Hash, Hasher};
use std::net::IpAddr;
use std::str::FromStr;
use time::format_description::well_known::Rfc3339;
use url::Url;

#[cfg(all(test, feature = "validation"))]
mod tests;

/// The array length up to which `uniqueItems` compares elements pairwise with the JSON
/// Schema validator's numeric equality; it compares a longer array's elements by their
/// JSON value instead.
const UNIQUE_ITEMS_PAIRWISE_LIMIT: usize = 15;

/// A date's digits, as the JSON Schema validator requires them besides the date parse,
/// which ignores missing zero padding.
static DATE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}\z").expect("a literal pattern compiles"));

/// A time, as the JSON Schema validator's `time` format reads one.
static TIME: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9])(\.[0-9]{6})?(([Zz])|([+|\-]([01][0-9]|2[0-3]):[0-5][0-9]))\z",
    )
    .expect("a literal pattern compiles")
});

/// A document type's validation schema ready to check document values: its patterns
/// compiled once.
#[derive(Debug)]
pub struct DocumentSchemaValidator {
    schema: ValidationSchema,
    patterns: BTreeMap<String, Regex>,
}

impl DocumentSchemaValidator {
    /// The validator of `schema`. Refuses a `pattern` the JSON Schema validator does not
    /// compile and a property declared twice, which the builder refuses already, and a chain
    /// of `$ref`s that returns to where it started without entering the value.
    pub fn new(schema: ValidationSchema) -> Result<Self, DataContractError> {
        let mut patterns = BTreeMap::new();
        {
            let mut nodes: Vec<&SchemaNode> = vec![&schema.root];
            nodes.extend(schema.references.values().map(|node| node.as_ref()));
            while let Some(node) = nodes.pop() {
                if let Some(pattern) = node.string.as_deref().and_then(|s| s.pattern.as_ref()) {
                    if !patterns.contains_key(pattern) {
                        let regex = validator_regex(pattern).ok_or_else(|| {
                            structure_error(format!(
                                "the JSON Schema validator compiles no `pattern` `{pattern}`"
                            ))
                        })?;
                        patterns.insert(pattern.clone(), regex);
                    }
                }
                if let Some(object) = node.object.as_deref() {
                    // The walk counts the declared properties a value holds against its keys
                    let mut names = BTreeSet::new();
                    for (name, property) in object.properties.iter().flatten() {
                        if !names.insert(name) {
                            return Err(structure_error(format!(
                                "the property `{name}` is declared twice"
                            )));
                        }
                        nodes.push(property);
                    }
                }
                if let Some(array) = node.array.as_deref() {
                    nodes.extend(array.items.as_deref());
                    nodes.extend(array.contains.as_deref());
                }
            }
        }
        // Each `$ref` is followed at the same value: a chain longer than the references
        // held returns to one of them, and entering it would consume nothing of the value
        let references = &schema.references;
        for (address, start) in references {
            let mut node: &SchemaNode = start;
            let mut followed = 0;
            while let Some(next) = node
                .reference
                .as_ref()
                .and_then(|reference| references.get(reference))
            {
                followed += 1;
                if followed > references.len() {
                    return Err(structure_error(format!(
                        "`$ref` `{address}` leads to a cycle of references that consumes \
                         nothing of the value"
                    )));
                }
                node = next;
            }
        }
        Ok(Self { schema, patterns })
    }

    /// The first violation of `instance`, the JSON the document validator converts a
    /// document's properties to, as the JSON Schema error that reports it: none when it
    /// satisfies the schema. Consensus reports only the first violation, so the walk stops
    /// there. Errs only on a cycle of `$ref`s that consumes nothing of the value, which
    /// [`Self::new`] refuses.
    pub fn validate(&self, instance: &JsonValue) -> Result<Option<JsonSchemaError>, ProtocolError> {
        let mut walk = Walk::new(self, false);
        walk.node(&self.schema.root, instance, 0)?;
        Ok(walk.violation)
    }
}

/// One check of a value against the schema, stopping at the first failure it meets.
struct Walk<'v> {
    validator: &'v DocumentSchemaValidator,
    /// Whether the walk only asks whether the value matches, as the JSON Schema validator
    /// asks of each element against a `contains` schema: it builds no violation, and skips
    /// `byteArray`, whose match check accepts every value.
    matching_only: bool,
    /// Whether the walk met a failure: it has its answer and checks nothing more.
    failed: bool,
    instance_path: Vec<Segment<'v>>,
    schema_path: Vec<Segment<'v>>,
    /// The length of `schema_path` at each `$ref` the walk is inside, innermost last.
    reference_sites: Vec<usize>,
    /// The failure met, as the JSON Schema error reporting it, unless the walk only asks
    /// whether the value matches.
    violation: Option<JsonSchemaError>,
}

/// A step of a path through the document or the schema, rendered only when a violation is
/// reported.
#[derive(Clone, Copy)]
enum Segment<'v> {
    Name(&'v str),
    Index(usize),
}

impl<'v> Walk<'v> {
    fn new(validator: &'v DocumentSchemaValidator, matching_only: bool) -> Self {
        Self {
            validator,
            matching_only,
            failed: false,
            instance_path: Vec::new(),
            schema_path: Vec::new(),
            reference_sites: Vec::new(),
            violation: None,
        }
    }

    /// Whether the walk has its answer.
    fn done(&self) -> bool {
        self.failed
    }

    /// A violation of `keyword`, whose parameters and message `details` gives: built only
    /// for the first failure, and only when the walk reports it.
    fn fail(
        &mut self,
        keyword: &'static str,
        property_name: &str,
        details: impl FnOnce() -> (JsonValue, String),
    ) {
        self.fail_under(keyword, keyword, property_name, details);
    }

    /// [`Walk::fail`] for a violation of `keyword` the schema holds under `schema_keyword`,
    /// as a `dependentRequired` entry reports a missing property as `required`.
    fn fail_under(
        &mut self,
        schema_keyword: &'static str,
        keyword: &'static str,
        property_name: &str,
        details: impl FnOnce() -> (JsonValue, String),
    ) {
        if self.failed {
            return;
        }
        self.failed = true;
        if self.matching_only {
            return;
        }
        self.schema_path.push(Segment::Name(schema_keyword));
        let schema_path = pointer(&self.schema_path);
        self.schema_path.pop();
        let (params, message) = details();
        self.report(keyword, schema_path, params, property_name, message);
    }

    fn report(
        &mut self,
        keyword: &'static str,
        schema_path: String,
        params: JsonValue,
        property_name: &str,
        message: String,
    ) {
        self.failed = true;
        self.violation = Some(JsonSchemaError::new(
            message,
            keyword.to_string(),
            pointer(&self.instance_path),
            schema_path,
            params.into(),
            property_name.to_string(),
        ));
    }

    /// Checks `instance` against `node`, reached through `references_in_a_row` `$ref`s
    /// since the walk last entered a value.
    fn node(
        &mut self,
        node: &'v SchemaNode,
        instance: &JsonValue,
        references_in_a_row: usize,
    ) -> Result<(), ProtocolError> {
        if let Some(reference) = &node.reference {
            let validator = self.validator;
            let references = &validator.schema.references;
            let target = references.get(reference).ok_or_else(|| {
                ProtocolError::CorruptedCodeExecution(format!(
                    "the validation schema holds no schema for `$ref` `{reference}`"
                ))
            })?;
            if references_in_a_row >= references.len() {
                return Err(ProtocolError::CorruptedCodeExecution(format!(
                    "`$ref` `{reference}` closes a cycle of references that consumes nothing \
                     of the value"
                )));
            }
            // The JSON Schema validator reports a target's violations as if the target sat
            // in the place of the `$ref`
            self.reference_sites.push(self.schema_path.len());
            self.node(target, instance, references_in_a_row + 1)?;
            self.reference_sites.pop();
            if self.done() {
                return Ok(());
            }
        }
        if let Some(types) = &node.types {
            if !types_admit(types, instance) {
                self.fail("type", "", || {
                    let names = match types {
                        JsonTypes::One(json_type) => json!(json_type.name()),
                        // A list of one type reads as that type
                        JsonTypes::AnyOf(json_types) => match json_types.as_slice() {
                            [json_type] => json!(json_type.name()),
                            // Each type once, in alphabetical order, as the JSON Schema
                            // validator lists them: `["string", "string"]` reads as
                            // `["string"]`
                            json_types => {
                                let mut names: Vec<&str> = json_types
                                    .iter()
                                    .map(|json_type| json_type.name())
                                    .collect();
                                names.sort_unstable();
                                names.dedup();
                                json!(names)
                            }
                        },
                    };
                    let message = format!("{instance} is not of type {names}");
                    (json!({ "type": names }), message)
                });
            }
        }
        if self.done() {
            return Ok(());
        }
        if let Some(values) = &node.enum_values {
            if !values.iter().any(|value| equal_value(instance, value)) {
                self.fail("enum", "", || {
                    let options = json_of_values(values);
                    let message = format!("{instance} is not one of {options}");
                    (json!({ "enum": options }), message)
                });
            }
        }
        if self.done() {
            return Ok(());
        }
        if let Some(constant) = &node.constant {
            if !const_admits(constant, instance) {
                self.fail("const", "", || {
                    let expected = json_of_value(constant);
                    let message = format!("{expected} was expected");
                    (json!({ "allowedValue": expected }), message)
                });
            }
        }
        if self.done() {
            return Ok(());
        }
        match instance {
            JsonValue::Object(map) => {
                if let Some(object) = node.object.as_deref() {
                    self.object(object, map)?;
                }
            }
            JsonValue::Array(items) => {
                if let Some(array) = node.array.as_deref() {
                    self.array(array, items)?;
                }
            }
            JsonValue::String(string) => {
                if let Some(constraints) = node.string.as_deref() {
                    self.string(constraints, string)?;
                }
                if self.done() {
                    return Ok(());
                }
                if let Some(format) = node.format.as_deref().and_then(|f| f.format) {
                    if !format_admits(format, string) {
                        self.fail("format", "", || {
                            let name = format.name();
                            (
                                json!({ "format": name }),
                                format!("{instance} is not a \"{name}\""),
                            )
                        });
                    }
                }
            }
            JsonValue::Number(number) => {
                if let Some(constraints) = node.number.as_deref() {
                    self.number(constraints, number)?;
                }
            }
            JsonValue::Null | JsonValue::Bool(_) => {}
        }
        if !self.matching_only
            && !self.done()
            && node.array.as_deref().is_some_and(|array| array.byte_array)
        {
            self.byte_array(instance);
        }
        Ok(())
    }

    fn object(
        &mut self,
        object: &'v ObjectConstraints,
        map: &JsonMap<String, JsonValue>,
    ) -> Result<(), ProtocolError> {
        // The declared properties the value holds: each name is declared once, so when they
        // are all its keys, it holds no other
        let mut declared_keys = 0;
        for (name, property) in object.properties.iter().flatten() {
            let Some(value) = map.get(name) else {
                continue;
            };
            declared_keys += 1;
            self.schema_path.push(Segment::Name("properties"));
            self.schema_path.push(Segment::Name(name));
            self.instance_path.push(Segment::Name(name));
            self.node(property, value, 0)?;
            self.instance_path.pop();
            self.schema_path.pop();
            self.schema_path.pop();
            if self.done() {
                return Ok(());
            }
        }
        // Fewer declared keys than keys: the value holds a key it does not declare, which
        // the violation lists only when it is reported. Without `properties` the JSON Schema
        // validator compiles `additionalProperties: false` as a false schema, whose one
        // violation carries no parameters.
        if object.additional_properties == Some(false) && declared_keys < map.len() {
            match &object.properties {
                None => self.fail_under("additionalProperties", "falseSchema", "", || {
                    let message = format!("False schema does not allow {}", json!(map));
                    (json!({}), message)
                }),
                Some(properties) => self.fail("additionalProperties", "", || {
                    let unexpected: Vec<&String> = map
                        .keys()
                        .filter(|key| !properties.iter().any(|(name, _)| name == *key))
                        .collect();
                    let message = format!(
                        "additional properties are not allowed ({unexpected:?} were unexpected)"
                    );
                    (json!({ "additionalProperties": unexpected }), message)
                }),
            }
        }
        if self.done() {
            return Ok(());
        }
        for name in object.required.iter().flatten() {
            if !map.contains_key(name) {
                self.fail("required", &reported_property_name(name), || {
                    (
                        json!({ "missingProperty": name }),
                        format!("\"{name}\" is a required property"),
                    )
                });
                return Ok(());
            }
        }
        let count = map.len() as u64;
        if let Some(minimum) = object.min_properties {
            if count < minimum {
                self.fail("minProperties", "", || {
                    (
                        json!({ "minProperties": minimum }),
                        format!("an object with fewer than {minimum} properties is not allowed"),
                    )
                });
            }
        }
        if let Some(maximum) = object.max_properties {
            if count > maximum {
                self.fail("maxProperties", "", || {
                    (
                        json!({ "maxProperties": maximum }),
                        format!("an object with more than {maximum} properties is not allowed"),
                    )
                });
            }
        }
        if self.done() {
            return Ok(());
        }
        for (name, dependents) in object.dependent_required.iter().flatten() {
            if !map.contains_key(name) {
                continue;
            }
            for dependent in dependents {
                if !map.contains_key(dependent) {
                    let reported = reported_property_name(dependent);
                    self.fail_under("dependentRequired", "required", &reported, || {
                        (
                            json!({ "missingProperty": dependent }),
                            format!("\"{dependent}\" is a required property"),
                        )
                    });
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn array(
        &mut self,
        array: &'v ArrayConstraints,
        items: &[JsonValue],
    ) -> Result<(), ProtocolError> {
        if let Some(element) = array.items.as_deref() {
            self.schema_path.push(Segment::Name("items"));
            for (index, item) in items.iter().enumerate() {
                self.instance_path.push(Segment::Index(index));
                self.node(element, item, 0)?;
                self.instance_path.pop();
                if self.done() {
                    break;
                }
            }
            self.schema_path.pop();
            if self.done() {
                return Ok(());
            }
        }
        if let Some(contains) = array.contains.as_deref() {
            let mut matched = false;
            for item in items {
                let mut walk = Walk::new(self.validator, true);
                walk.node(contains, item, 0)?;
                if !walk.failed {
                    matched = true;
                    break;
                }
            }
            if !matched {
                self.fail("contains", "", || {
                    (
                        json!({}),
                        "none of the items are valid under the given schema".to_string(),
                    )
                });
                return Ok(());
            }
        }
        let count = items.len() as u64;
        if let Some(minimum) = array.min_items {
            if count < minimum {
                self.fail("minItems", "", || {
                    (
                        json!({ "minItems": minimum }),
                        format!("an array with fewer than {minimum} items is not allowed"),
                    )
                });
            }
        }
        if let Some(maximum) = array.max_items {
            if count > maximum {
                self.fail("maxItems", "", || {
                    (
                        json!({ "maxItems": maximum }),
                        format!("an array with more than {maximum} items is not allowed"),
                    )
                });
            }
        }
        if !self.done() && array.unique_items == Some(true) && !is_unique(items) {
            self.fail("uniqueItems", "", || {
                (json!({}), "the array has non-unique elements".to_string())
            });
        }
        Ok(())
    }

    /// `byteArray`, as Platform's keyword checks a value where errors are reported: an
    /// array whose elements each read as an unsigned integer up to 255, stopping at the
    /// first that does not. The keyword reports no schema path of its own: a violation
    /// carries only the path of the `$ref`s the walk followed to it, which the JSON Schema
    /// validator adds as it returns through each.
    fn byte_array(&mut self, instance: &JsonValue) {
        let failure = match instance {
            JsonValue::Array(items) => {
                items
                    .iter()
                    .enumerate()
                    .find_map(|(index, item)| match item.as_u64() {
                        Some(byte) if byte > u64::from(u8::MAX) => Some((
                            Some(index),
                            "maximum",
                            json!({ "maximum": u8::MAX }),
                            format!("{item} is greater than the maximum of {}", u8::MAX),
                        )),
                        Some(_) => None,
                        None => Some((
                            Some(index),
                            "type",
                            json!({ "type": "integer" }),
                            format!("{item} is not of type \"integer\""),
                        )),
                    })
            }
            _ => Some((
                None,
                "type",
                json!({ "type": "array" }),
                format!("{instance} is not of type \"array\""),
            )),
        };
        let Some((index, keyword, params, message)) = failure else {
            return;
        };
        let schema_path = self
            .reference_sites
            .last()
            .and_then(|&site| self.schema_path.get(..site))
            .map(pointer)
            .unwrap_or_default();
        if let Some(index) = index {
            self.instance_path.push(Segment::Index(index));
        }
        self.report(keyword, schema_path, params, "", message);
        if index.is_some() {
            self.instance_path.pop();
        }
    }

    fn string(&mut self, string: &StringConstraints, value: &str) -> Result<(), ProtocolError> {
        let length = if string.min_length.is_some() || string.max_length.is_some() {
            value.chars().count() as u64
        } else {
            0
        };
        if let Some(minimum) = string.min_length {
            if length < minimum {
                self.fail("minLength", "", || {
                    (
                        json!({ "minLength": minimum }),
                        format!("\"{value}\" is shorter than {minimum} characters"),
                    )
                });
            }
        }
        if let Some(maximum) = string.max_length {
            if length > maximum {
                self.fail("maxLength", "", || {
                    (
                        json!({ "maxLength": maximum }),
                        format!("\"{value}\" is longer than {maximum} characters"),
                    )
                });
            }
        }
        if self.done() {
            return Ok(());
        }
        if let Some(pattern) = &string.pattern {
            let regex = self.validator.patterns.get(pattern).ok_or_else(|| {
                ProtocolError::CorruptedCodeExecution(format!(
                    "the validator compiled no `pattern` `{pattern}`"
                ))
            })?;
            if !regex.is_match(value) {
                self.fail("pattern", "", || {
                    (
                        json!({ "pattern": pattern }),
                        format!("\"{value}\" does not match \"{pattern}\""),
                    )
                });
            }
        }
        Ok(())
    }

    fn number(
        &mut self,
        number: &NumberConstraints,
        value: &JsonNumber,
    ) -> Result<(), ProtocolError> {
        let item = Num::of_json(value);
        let bounds: [Bound; 4] = [
            (
                &number.minimum,
                "minimum",
                |item, limit| !num_lt(item, limit),
                "less than the minimum of",
            ),
            (
                &number.maximum,
                "maximum",
                |item, limit| !num_gt(item, limit),
                "greater than the maximum of",
            ),
            (
                &number.exclusive_minimum,
                "exclusiveMinimum",
                num_gt,
                "less than or equal to the minimum of",
            ),
            (
                &number.exclusive_maximum,
                "exclusiveMaximum",
                num_lt,
                "greater than or equal to the maximum of",
            ),
        ];
        for (bound, keyword, admits, relation) in bounds {
            let Some(bound) = bound else {
                continue;
            };
            let limit = Num::of_value(bound).ok_or_else(|| not_a_number(keyword))?;
            if !admits(item, limit) {
                self.fail(keyword, "", || {
                    let limit = json_of_value(bound);
                    let message = format!("{value} is {relation} {limit}");
                    (json!({ keyword: limit }), message)
                });
                return Ok(());
            }
        }
        if let Some(divisor) = &number.multiple_of {
            let divisor = Num::of_value(divisor)
                .ok_or_else(|| not_a_number("multipleOf"))?
                .as_f64();
            if !is_multiple_of(value_as_f64(value), divisor) {
                self.fail("multipleOf", "", || {
                    (
                        json!({ "multipleOf": divisor }),
                        format!("{value} is not a multiple of {divisor}"),
                    )
                });
            }
        }
        Ok(())
    }
}

/// A bound of a number: the value the schema holds, its keyword, whether an item within
/// it passes, and how an item outside it relates to it.
type Bound<'n> = (
    &'n Option<Value>,
    &'static str,
    fn(Num, Num) -> bool,
    &'static str,
);

fn not_a_number(keyword: &str) -> ProtocolError {
    ProtocolError::CorruptedCodeExecution(format!(
        "the validation schema holds a `{keyword}` that is not a number"
    ))
}

/// A property name as the JSON Schema validator's error names it: written as a JSON string,
/// without its quotes.
fn reported_property_name(name: &str) -> String {
    JsonValue::String(name.to_string())
        .to_string()
        .trim_matches('"')
        .to_string()
}

/// A JSON pointer of `segments`.
fn pointer(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|segment| match segment {
            Segment::Name(name) => format!("/{}", name.replace('~', "~0").replace('/', "~1")),
            Segment::Index(index) => format!("/{index}"),
        })
        .collect()
}

/// Whether `instance` is of one of `types`: a number is an integer when it reads as one or
/// as a float with no fractional part.
fn types_admit(types: &JsonTypes, instance: &JsonValue) -> bool {
    let admits = |json_type: &JsonType| match (json_type, instance) {
        (JsonType::Null, JsonValue::Null)
        | (JsonType::Boolean, JsonValue::Bool(_))
        | (JsonType::Object, JsonValue::Object(_))
        | (JsonType::Array, JsonValue::Array(_))
        | (JsonType::String, JsonValue::String(_))
        | (JsonType::Number, JsonValue::Number(_)) => true,
        (JsonType::Integer, JsonValue::Number(number)) => {
            number.is_u64() || number.is_i64() || value_as_f64(number).fract() == 0.
        }
        _ => false,
    };
    match types {
        JsonTypes::One(json_type) => admits(json_type),
        JsonTypes::AnyOf(json_types) => json_types.iter().any(admits),
    }
}

/// A JSON number, read as the JSON Schema validator reads one: unsigned when it can, else
/// signed, else a float.
#[derive(Clone, Copy)]
enum Num {
    U(u64),
    I(i64),
    F(f64),
}

impl Num {
    fn of_json(number: &JsonNumber) -> Self {
        if let Some(number) = number.as_u64() {
            Num::U(number)
        } else if let Some(number) = number.as_i64() {
            Num::I(number)
        } else {
            Num::F(value_as_f64(number))
        }
    }

    /// A number the validation schema holds, as the validator reads it from the JSON the
    /// schema converts to; `None` for any other value.
    fn of_value(value: &Value) -> Option<Self> {
        match value {
            Value::U64(number) => Some(Num::U(*number)),
            Value::U32(number) => Some(Num::U(u64::from(*number))),
            Value::U16(number) => Some(Num::U(u64::from(*number))),
            Value::U8(number) => Some(Num::U(u64::from(*number))),
            Value::I64(number) => Some(Num::signed(*number)),
            Value::I32(number) => Some(Num::signed(i64::from(*number))),
            Value::I16(number) => Some(Num::signed(i64::from(*number))),
            Value::I8(number) => Some(Num::signed(i64::from(*number))),
            Value::Float(number) => Some(Num::F(*number)),
            _ => None,
        }
    }

    fn signed(number: i64) -> Self {
        match u64::try_from(number) {
            Ok(number) => Num::U(number),
            Err(_) => Num::I(number),
        }
    }

    fn as_f64(self) -> f64 {
        match self {
            Num::U(number) => number as f64,
            Num::I(number) => number as f64,
            Num::F(number) => number,
        }
    }
}

/// A JSON number as an f64; every number of the JSON the validator reads has one.
fn value_as_f64(number: &JsonNumber) -> f64 {
    number.as_f64().unwrap_or(f64::NAN)
}

macro_rules! num_relation {
    ($name:ident, $method:ident) => {
        fn $name(left: Num, right: Num) -> bool {
            match (left, right) {
                (Num::U(left), Num::U(right)) => NumCmp::$method(left, right),
                (Num::U(left), Num::I(right)) => NumCmp::$method(left, right),
                (Num::U(left), Num::F(right)) => NumCmp::$method(left, right),
                (Num::I(left), Num::U(right)) => NumCmp::$method(left, right),
                (Num::I(left), Num::I(right)) => NumCmp::$method(left, right),
                (Num::I(left), Num::F(right)) => NumCmp::$method(left, right),
                (Num::F(left), Num::U(right)) => NumCmp::$method(left, right),
                (Num::F(left), Num::I(right)) => NumCmp::$method(left, right),
                (Num::F(left), Num::F(right)) => NumCmp::$method(left, right),
            }
        }
    };
}

num_relation!(num_eq, num_eq);
num_relation!(num_lt, num_lt);
num_relation!(num_gt, num_gt);

/// Whether `item` is a multiple of `divisor`, as the JSON Schema validator decides it in
/// f64: under an integral divisor, an item with no fractional part and no remainder; under
/// a fractional one, an item whose quotient's remainder modulo 1 is below `f64::EPSILON`,
/// or, when that quotient overflows, whose quotient `fraction` reads as a whole number.
fn is_multiple_of(item: f64, divisor: f64) -> bool {
    if divisor.fract() == 0. {
        return item.fract() == 0. && (item % divisor) == 0.;
    }
    let remainder = (item / divisor) % 1.;
    if remainder.is_nan() {
        let quotient = BigFraction::from(item) / BigFraction::from(divisor);
        match quotient.denom() {
            Some(denominator) => denominator == &BigUint::from(1_u8),
            None => true,
        }
    } else {
        remainder < f64::EPSILON
    }
}

/// Whether `instance` satisfies `const: constant`: a number within `f64::EPSILON` of a
/// number `const`, any other value equal to it ([`equal_value`]).
fn const_admits(constant: &Value, instance: &JsonValue) -> bool {
    match Num::of_value(constant) {
        Some(constant) => match instance {
            JsonValue::Number(item) => {
                (constant.as_f64() - value_as_f64(item)).abs() < f64::EPSILON
            }
            _ => false,
        },
        None => equal_value(instance, constant),
    }
}

/// Whether `instance` equals `expected`, a value the validation schema holds, as the JSON
/// Schema validator compares an `enum` member: numbers by value whatever their reading,
/// arrays element by element, objects entry by entry in written order.
fn equal_value(instance: &JsonValue, expected: &Value) -> bool {
    match (instance, expected) {
        (JsonValue::String(instance), Value::Text(expected)) => instance == expected,
        (JsonValue::Bool(instance), Value::Bool(expected)) => instance == expected,
        (JsonValue::Null, Value::Null) => true,
        (JsonValue::Number(instance), expected) => {
            Num::of_value(expected).is_some_and(|expected| num_eq(Num::of_json(instance), expected))
        }
        (JsonValue::Array(instance), Value::Array(expected)) => {
            instance.len() == expected.len()
                && instance
                    .iter()
                    .zip(expected)
                    .all(|(instance, expected)| equal_value(instance, expected))
        }
        (JsonValue::Object(instance), Value::Map(expected)) => {
            instance.len() == expected.len()
                && instance
                    .iter()
                    .zip(expected)
                    .all(|((key, instance), (name, expected))| {
                        name.as_text() == Some(key.as_str()) && equal_value(instance, expected)
                    })
        }
        _ => false,
    }
}

/// Whether two values of the document are equal as [`equal_value`] compares.
fn equal_json(left: &JsonValue, right: &JsonValue) -> bool {
    match (left, right) {
        (JsonValue::String(left), JsonValue::String(right)) => left == right,
        (JsonValue::Bool(left), JsonValue::Bool(right)) => left == right,
        (JsonValue::Null, JsonValue::Null) => true,
        (JsonValue::Number(left), JsonValue::Number(right)) => {
            num_eq(Num::of_json(left), Num::of_json(right))
        }
        (JsonValue::Array(left), JsonValue::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| equal_json(left, right))
        }
        (JsonValue::Object(left), JsonValue::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|((left_key, left), (right_key, right))| {
                        left_key == right_key && equal_json(left, right)
                    })
        }
        _ => false,
    }
}

/// Whether no two elements of `items` are equal: up to
/// [`UNIQUE_ITEMS_PAIRWISE_LIMIT`] elements as [`equal_json`] compares, more by their
/// JSON value, where numbers of one reading compare by value, `1` and `1.0` differ, and
/// objects with the same entries are equal whatever their order. `0.0` and `-0.0` are one
/// value at every length, a rule of protocol version 15.
fn is_unique(items: &[JsonValue]) -> bool {
    if items.len() <= UNIQUE_ITEMS_PAIRWISE_LIMIT {
        return items.iter().enumerate().all(|(index, item)| {
            items
                .iter()
                .skip(index + 1)
                .all(|other| !equal_json(item, other))
        });
    }
    // The first repeat ends the check; equality, not the hash, decides what repeats
    let mut seen = HashSet::with_capacity(items.len());
    items.iter().all(|item| seen.insert(UniqueElement(item)))
}

/// An element of a list `uniqueItems` checks beyond [`UNIQUE_ITEMS_PAIRWISE_LIMIT`]: equal to
/// another as JSON values are, hashed as [`hash_json`] hashes it.
struct UniqueElement<'a>(&'a JsonValue);

impl Hash for UniqueElement<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_json(self.0, state);
    }
}

impl PartialEq for UniqueElement<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for UniqueElement<'_> {}

/// Hashes `value` so that two values equal as JSON values hash alike.
fn hash_json<H: Hasher>(value: &JsonValue, state: &mut H) {
    match value {
        JsonValue::Null => state.write_u8(0),
        JsonValue::Bool(value) => {
            state.write_u8(1);
            value.hash(state);
        }
        JsonValue::Number(number) => {
            state.write_u8(2);
            if let Some(number) = number.as_u64() {
                state.write_u8(0);
                number.hash(state);
            } else if let Some(number) = number.as_i64() {
                state.write_u8(1);
                number.hash(state);
            } else {
                let number = value_as_f64(number);
                state.write_u8(2);
                // `-0.0` equals `0.0`
                (if number == 0. { 0_f64 } else { number })
                    .to_bits()
                    .hash(state);
            }
        }
        JsonValue::String(value) => {
            state.write_u8(3);
            value.hash(state);
        }
        JsonValue::Array(items) => {
            state.write_u8(4);
            items.len().hash(state);
            for item in items {
                hash_json(item, state);
            }
        }
        JsonValue::Object(map) => {
            state.write_u8(5);
            map.len().hash(state);
            // Entries in any order: a sum of their own hashes
            let entries = map.iter().fold(0_u64, |sum, (key, value)| {
                let mut entry = DefaultHasher::new();
                key.hash(&mut entry);
                hash_json(value, &mut entry);
                sum.wrapping_add(entry.finish())
            });
            entries.hash(state);
        }
    }
}

/// Whether `value` follows `format` as the JSON Schema validator reads it.
fn format_admits(format: ValidatorFormat, value: &str) -> bool {
    match format {
        ValidatorFormat::Date => {
            time::Date::parse(
                value,
                &time::macros::format_description!("[year]-[month]-[day]"),
            )
            .is_ok()
                && DATE.is_match(value)
        }
        ValidatorFormat::DateTime => time::OffsetDateTime::parse(value, &Rfc3339).is_ok(),
        ValidatorFormat::Email | ValidatorFormat::IdnEmail => is_email(value),
        ValidatorFormat::Hostname => is_hostname(value),
        ValidatorFormat::Ipv4 => {
            !value.starts_with('0')
                && IpAddr::from_str(value).is_ok_and(|address| address.is_ipv4())
        }
        ValidatorFormat::Ipv6 => IpAddr::from_str(value).is_ok_and(|address| address.is_ipv6()),
        // Under the `regex` crate's default size limit, as generation 1 compiles a document's
        // `format: regex` value: document validation checks a contract read from state, whose
        // validator `compile_and_validate` builds with the default engine. The 5 MiB limit of
        // `JsonSchemaValidator::compile` applies only where registration compiles the schema.
        ValidatorFormat::Regex => validator_regex(value).is_some(),
        ValidatorFormat::Time => TIME.is_match(value),
        ValidatorFormat::Uri => Url::from_str(value).is_ok(),
    }
}

/// An email address as the JSON Schema validator checks one: a local part that neither
/// starts nor ends with a dot nor holds two in a row, before an `@`; the domain is not
/// read.
fn is_email(value: &str) -> bool {
    if value.starts_with('.') {
        return false;
    }
    for (current, next) in value.chars().zip(value.chars().skip(1)) {
        match (current, next) {
            ('.', '.') | ('.', '@') => return false,
            (_, '@') => return true,
            _ => continue,
        }
    }
    false
}

/// A host name as the JSON Schema validator checks one.
fn is_hostname(value: &str) -> bool {
    !(value.ends_with('-')
        || value.starts_with('-')
        || value.is_empty()
        || value.chars().count() > 255
        || value
            .chars()
            .any(|c| !(c.is_alphanumeric() || c == '-' || c == '.'))
        || value.split('.').any(|part| part.chars().count() > 63))
}

/// `value`, a value the validation schema holds, as JSON, for an error's parameters.
fn json_of_value(value: &Value) -> JsonValue {
    value.try_to_validating_json().unwrap_or(JsonValue::Null)
}

fn json_of_values(values: &[Value]) -> JsonValue {
    JsonValue::Array(values.iter().map(json_of_value).collect())
}
