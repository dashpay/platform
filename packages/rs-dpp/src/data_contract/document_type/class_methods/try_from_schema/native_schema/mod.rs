//! The schema checks of `check_document_type_schema` 1 (protocol version 15), which full
//! validation runs at registration in place of the document meta-schema
//! (`schema/meta_schemas/document/v3/document-meta.json`), which the `jsonschema` crate
//! checked the enriched schema against through protocol version 14.
//!
//! [`check_document_type_schema`] repeats that meta-schema's rules, read from the JSON the
//! crate checked, so a repeated key reads as its last entry and every value as its JSON
//! form, with the crate's readings of an integer (a float with no fractional part is one),
//! a number bound (exact across integer and float readings), a `const` number (within
//! `f64::EPSILON`) and an array of unique items. A schema breaking one is refused with the
//! [`JsonSchemaError`] the meta-schema check returned, so the error code stays. Where the
//! meta-schema leaves a schema unchecked, a `contains` or a system property's, it also refuses
//! a `pattern` that does not compile under the regex size limit the crate compiled the whole
//! schema under once the meta-schema check passed.
//!
//! Four rules are protocol version 15's own. An index property name is at most 255 bytes, the
//! longest key GroveDB stores, where the meta-schema took 256 characters. In a list that
//! must be unique, such as an `enum`'s members, `0.0` and `-0.0` are one value however long
//! the list, as the native document validator compares them. A document schema may
//! declare `private` (`{ "entrySize": n }`, an integer from 1 and no other key), which the
//! meta-schema refused as an unknown key: a private document type, read by
//! `parse_private_document` 0. Every version selecting this check parses the key (a test over
//! the version tables holds them to it), so a type is never registered public with a schema
//! saying it is private. And no object of a document schema as sent, at its root or below
//! it, or of the contract's `$defs`, names a key twice ([`refuse_a_repeated_doctype_key`]),
//! which the meta-schema could not see: the JSON it checked kept only the last entry of a
//! repeated key.
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
    equal_json, is_unique, validator_regex_within, DocumentSchemaValidator,
};
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use num_cmp::NumCmp;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use serde_json::{json, Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use std::collections::{BTreeMap, BTreeSet};

mod constraints;
mod document_type;
mod indexes;
mod properties;
mod references;
#[cfg(test)]
mod tests;

/// The keys a document schema may hold, for the tests guarding what reads them.
#[cfg(test)]
pub(in crate::data_contract) use document_type::ROOT_KEYS as DOCUMENT_SCHEMA_ROOT_KEYS;

/// The meta-schema's pattern of a name: 1 to 64 ASCII letters, digits and underscores.
const WORD_PATTERN: &str = "^[a-zA-Z0-9_]{1,64}$";

/// The meta-schema's pattern of a property path: names joined by dots.
const PATH_PATTERN: &str = "^[a-zA-Z0-9_]{1,64}(\\.[a-zA-Z0-9_]{1,64})*$";

/// The meta-schema's pattern of a property path or `$ownerId`.
const OWNER_OR_PATH_PATTERN: &str = "^(\\$ownerId|[a-zA-Z0-9_]{1,64}(\\.[a-zA-Z0-9_]{1,64})*)$";

/// The meta-schema's pattern of a property path, `$ownerId` or `$id`.
const OWNER_ID_OR_PATH_PATTERN: &str =
    "^(\\$ownerId|\\$id|[a-zA-Z0-9_]{1,64}(\\.[a-zA-Z0-9_]{1,64})*)$";

/// The meta-schema's pattern of what a rule may read of a referenced document.
const REFERENCED_PROPERTY_PATTERN: &str = "^(\\$id|\\$ownerId|\\$creatorId|\\$(created|updated|transferred)At(BlockHeight|CoreBlockHeight)?|[a-zA-Z0-9_]{1,64}(\\.[a-zA-Z0-9_]{1,64})*)$";

/// The system timestamps a rule or a `where` bound may read.
const TIMESTAMP_PROPERTIES: [&str; 9] = [
    "$createdAt",
    "$updatedAt",
    "$transferredAt",
    "$createdAtBlockHeight",
    "$updatedAtBlockHeight",
    "$transferredAtBlockHeight",
    "$createdAtCoreBlockHeight",
    "$updatedAtCoreBlockHeight",
    "$transferredAtCoreBlockHeight",
];

/// Refuses a key an object of the document `schema` of the type `name`, or of the contract's
/// `schema_defs`, names more than once, at the schema's root or anywhere below it, as full
/// validation does under `check_document_type_schema` 1. Run on the schema and `$defs` as
/// sent, before the parser expands their property type shorthands (which keep the last of
/// a repeated `type` or `size` and drop the rest) and before any check reads them as JSON,
/// which keeps only the last entry of a repeated key. The document type parser reads the
/// first entry of many a repeated key (every document type keyword at the root, `required`
/// and `transient` down a nested `properties`, an index's keys, a moderator ability), where
/// the schema checks and the document validator read the last, and its own walks disagree
/// below the root (the `required` walk reads the first `properties` of a nested object, the
/// property walk the last), so the type the chain parsed could differ from the one its schema
/// shows. A private type's schema root is held to the rule on every parse
/// (`declares_private_document`); this one holds at registration only, so a contract stored
/// before protocol version 15 still loads as it was written. The walk is linear in the
/// schema's size and recurses no deeper than decoding the schema did. It reads every value,
/// whatever a `$ref` or annotation in it says, and counts in `read` each value it reads,
/// for the caller to bill on a refusal.
pub(super) fn refuse_a_repeated_doctype_key(
    schema: &Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    name: &str,
    read: &mut u64,
) -> Result<(), ProtocolError> {
    refuse_a_repeated_key_within(schema, name, &mut String::new(), read)?;
    for (definition, value) in schema_defs.into_iter().flatten() {
        refuse_a_repeated_key_within(value, name, &mut format!("$defs.{definition}"), read)?;
    }
    Ok(())
}

/// [`refuse_a_repeated_doctype_key`] over `value`, found at `path` within the schema (empty
/// at its root), and everything it holds.
fn refuse_a_repeated_key_within(
    value: &Value,
    name: &str,
    path: &mut String,
    read: &mut u64,
) -> Result<(), ProtocolError> {
    *read = read.saturating_add(1);
    match value {
        Value::Map(map) => {
            let mut declared = BTreeSet::new();
            for (key, entry) in map {
                // A key that is not text is refused by the checks that follow; its entry is
                // read here, so it is counted, but not walked into
                let Some(key) = key.as_text() else {
                    *read = read.saturating_add(1);
                    continue;
                };
                if !declared.insert(key) {
                    let place = if path.is_empty() {
                        "at its root".to_string()
                    } else {
                        format!("at `{path}`")
                    };
                    return Err(consensus_or_protocol_data_contract_error(
                        DataContractError::InvalidContractStructure(format!(
                            "document type \"{name}\" declares `{key}` more than once {place}: \
                             the document type parser may read its first entry where the \
                             schema checks read its last"
                        )),
                    ));
                }
                let len = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(key);
                refuse_a_repeated_key_within(entry, name, path, read)?;
                path.truncate(len);
            }
            Ok(())
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let len = path.len();
                path.push_str(&format!("[{index}]"));
                refuse_a_repeated_key_within(item, name, path, read)?;
                path.truncate(len);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Checks `root`, a document type's schema with the contract's `$defs` and the base schema
/// added and its property type shorthands expanded, against the rules of the document
/// meta-schema, as full validation does under `check_document_type_schema` 1.
pub(super) fn check_document_type_schema(
    root: &Value,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    let root = root.try_to_validating_json().map_err(|e| {
        ProtocolError::ConsensusError(
            ConsensusError::BasicError(BasicError::ValueError(e.into())).into(),
        )
    })?;
    Checker::new(platform_version).root(&root)
}

/// Requires the type whose expanded schema is `schema`, with the contract's expanded
/// `$defs`, to have a typed validation schema, refusing one it does not represent. Returns the
/// native validator built of it, which the type keeps.
pub(super) fn check_validation_schema(
    schema: &Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    platform_version: &PlatformVersion,
) -> Result<DocumentSchemaValidator, ProtocolError> {
    let root = DocumentType::enrich_with_base_schema(
        schema.clone(),
        schema_defs.map(|defs| Value::from(defs.clone())),
        platform_version,
    )?;
    // Document validation refuses the documents of a type whose validator does not build, a
    // cycle of references that consumes nothing included: registration refuses the type
    DocumentSchemaValidator::try_from_root(&root, platform_version).map_err(|error| match error {
        ProtocolError::DataContractError(refusal) => {
            consensus_or_protocol_data_contract_error(refusal)
        }
        error => error,
    })
}

/// A check of one value of a document schema, knowing where the value is.
struct Checker<'a> {
    /// The check of the value holding this one, and this value's key or index in it.
    parent: Option<(&'a Checker<'a>, String)>,
    /// The longest index property name, in bytes (`max_index_property_name_bytes`).
    max_index_property_name_bytes: usize,
}

type Checked = Result<(), ProtocolError>;

impl Checker<'_> {
    /// The check of a document schema, under the limits of `platform_version`.
    fn new(platform_version: &PlatformVersion) -> Self {
        Self {
            parent: None,
            max_index_property_name_bytes: usize::from(
                platform_version.system_limits.max_index_property_name_bytes,
            ),
        }
    }
}

impl Checker<'_> {
    /// Runs `check` on the member `segment` of the value in hand.
    fn at<T>(
        &self,
        segment: impl ToString,
        check: impl FnOnce(&Checker<'_>) -> Result<T, ProtocolError>,
    ) -> Result<T, ProtocolError> {
        check(&Checker {
            parent: Some((self, segment.to_string())),
            max_index_property_name_bytes: self.max_index_property_name_bytes,
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

    /// The error of a value of none of `types`, with the parameter the meta-schema check gave:
    /// the type, or the list of them in alphabetical order.
    fn type_error(&self, value: &JsonValue, types: &[&str]) -> ProtocolError {
        let mut types = types.to_vec();
        types.sort_unstable();
        let (described, param) = match types.as_slice() {
            [single] => (format!("\"{single}\""), json!(single)),
            types => (
                types
                    .iter()
                    .map(|single| format!("\"{single}\""))
                    .collect::<Vec<_>>()
                    .join(", "),
                json!(types),
            ),
        };
        let plural = if types.len() == 1 { "" } else { "s" };
        self.fail_with(
            "type",
            format!("{value} is not of type{plural} {described}"),
            json!({ "type": param }),
            "",
        )
    }

    /// The error of a string that does not match `pattern`, one of the meta-schema's.
    fn pattern_error(&self, value: &JsonValue, pattern: &str) -> ProtocolError {
        self.fail_with(
            "pattern",
            format!("{value} does not match \"{pattern}\""),
            json!({ "pattern": pattern }),
            "",
        )
    }

    /// The error of a value the meta-schema's `not` refuses, with the schema it negates.
    fn not_error(&self, negated: JsonValue, message: &str) -> ProtocolError {
        self.fail_with("not", message, json!({ "not": negated }), "")
    }

    // ---- primitives, as the meta-schema check read them ----

    fn object<'v>(
        &self,
        value: &'v JsonValue,
    ) -> Result<&'v JsonMap<String, JsonValue>, ProtocolError> {
        value
            .as_object()
            .ok_or_else(|| self.type_error(value, &["object"]))
    }

    fn array<'v>(&self, value: &'v JsonValue) -> Result<&'v Vec<JsonValue>, ProtocolError> {
        value
            .as_array()
            .ok_or_else(|| self.type_error(value, &["array"]))
    }

    fn string<'v>(&self, value: &'v JsonValue) -> Result<&'v str, ProtocolError> {
        value
            .as_str()
            .ok_or_else(|| self.type_error(value, &["string"]))
    }

    fn boolean(&self, value: &JsonValue) -> Checked {
        if value.is_boolean() {
            Ok(())
        } else {
            Err(self.type_error(value, &["boolean"]))
        }
    }

    fn number(&self, value: &JsonValue) -> Checked {
        if value.is_number() {
            Ok(())
        } else {
            Err(self.type_error(value, &["number"]))
        }
    }

    /// An integer, as the meta-schema check read one, from `minimum` to `maximum`.
    fn integer(&self, value: &JsonValue, minimum: Option<i64>, maximum: Option<i64>) -> Checked {
        let Some(number) = value.as_number().filter(|number| is_integer(number)) else {
            return Err(self.type_error(value, &["integer"]));
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

    /// No key of `map` outside `allowed`, as the meta-schema's `additionalProperties: false`.
    fn closed(&self, map: &JsonMap<String, JsonValue>, allowed: &[&str]) -> Checked {
        let unexpected = unexpected_keys(map, allowed);
        if unexpected.is_empty() {
            return Ok(());
        }
        Err(self.fail_with(
            "additionalProperties",
            format!("additional properties are not allowed ({unexpected:?} were unexpected)"),
            json!({ "additionalProperties": unexpected }),
            "",
        ))
    }

    /// No key of `map` outside `allowed`, as the meta-schema's `unevaluatedProperties: false`,
    /// which closes a property schema.
    fn closed_to_unevaluated(&self, map: &JsonMap<String, JsonValue>, allowed: &[&str]) -> Checked {
        let unexpected = unexpected_keys(map, allowed);
        if unexpected.is_empty() {
            return Ok(());
        }
        Err(self.fail_with(
            "unevaluatedProperties",
            format!("unevaluated properties are not allowed ({unexpected:?} were unexpected)"),
            json!({ "unexpected": unexpected }),
            "",
        ))
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
            Err(self.pattern_error(value, WORD_PATTERN))
        }
    }

    /// A property path: words joined by dots, at most 256 characters.
    fn path(&self, value: &JsonValue) -> Checked {
        let text = self.string_of_length(value, 1, Some(256))?;
        if is_path(text) {
            Ok(())
        } else {
            Err(self.pattern_error(value, PATH_PATTERN))
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

    // ---- names and choices ----

    /// The meta-schema's `propertyNames` error on the object in hand: its key `name` breaks
    /// `keyword` with `params`. The error of the name alone is nested as the meta-schema
    /// check nested it, but for the meta-schema location it also gave.
    fn name_error(
        &self,
        name: &str,
        keyword: &str,
        params: JsonValue,
        message: String,
    ) -> ProtocolError {
        self.fail_with(
            "propertyNames",
            message,
            json!({
                "instancePath": self.instance_path(),
                "instance": name,
                "params": params,
                "keyword": keyword,
                "propertyName": "",
            }),
            "",
        )
    }

    /// `name`, a key of the object in hand, of `minimum` to `maximum` characters.
    fn name_of_length(&self, name: &str, minimum: usize, maximum: usize) -> Checked {
        let length = name.chars().count();
        if length < minimum {
            return Err(self.name_error(
                name,
                "minLength",
                json!({ "minLength": minimum }),
                format!("\"{name}\" is shorter than {minimum} characters"),
            ));
        }
        if length > maximum {
            return Err(self.name_error(
                name,
                "maxLength",
                json!({ "maxLength": maximum }),
                format!("\"{name}\" is longer than {maximum} characters"),
            ));
        }
        Ok(())
    }

    /// `name`, a key of the object in hand, matching `pattern`, which `matches` says.
    fn name_matching(&self, name: &str, pattern: &str, matches: bool) -> Checked {
        if matches {
            return Ok(());
        }
        Err(self.name_error(
            name,
            "pattern",
            json!({ "pattern": pattern }),
            format!("\"{name}\" does not match \"{pattern}\""),
        ))
    }

    /// The error of a value no branch of a choice admits.
    fn no_branch(&self, value: &JsonValue) -> ProtocolError {
        self.fail(
            "anyOf",
            format!("{value} is not valid under any of the given schemas"),
        )
    }

    /// A string of 1 to 256 characters that `admits`.
    fn short_string_admitted(&self, value: &JsonValue, admits: impl Fn(&str) -> bool) -> bool {
        value
            .as_str()
            .is_some_and(|text| (1..=256).contains(&text.chars().count()) && admits(text))
    }
}

/// The keys of `map` outside `allowed`.
fn unexpected_keys<'m>(map: &'m JsonMap<String, JsonValue>, allowed: &[&str]) -> Vec<&'m String> {
    map.keys()
        .filter(|key| !allowed.contains(&key.as_str()))
        .collect()
}

/// Words joined by dots, as the meta-schema's property path pattern reads them.
fn is_path(text: &str) -> bool {
    text.split('.').all(is_word)
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
