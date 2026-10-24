//! A document type's JSON schema as the document validator reads it, typed.
//!
//! The document validator compiles a document type's schema with the contract's `$defs`
//! and the base schema added and its property type shorthands expanded, and checks every
//! document value against it with JSON Schema (draft 2020-12) semantics. Consensus also
//! checks two keywords of that schema that bound one value but that JSON Schema leaves
//! alone: `maxBytes` on a string, which the document validator checks right after, and
//! `encryptedFor` on a byte array, which the validation of a create or replace checks.
//! Both are read from the property the document type parser builds, so only where the
//! parser reads a schema as a property, and `maxBytes` also on the `items` of one, the
//! element schema of a typed array: not beside a property's `$ref`, whose referenced
//! schema the parser reads in its place, not in a schema only the JSON Schema validator
//! reads (a definition reached through a referenced schema's own `$ref`, say), and not
//! inside `contains`. Platform's `byteArray` keyword, which the JSON Schema validator checks
//! where it reports errors, constrains nothing inside `contains`, where the validator only
//! asks whether an element matches. The builder refuses each of these three keywords where
//! consensus does not check it.
//! [`ValidationSchema`] holds exactly the keywords of that schema that constrain a value on
//! its own, both kinds, one typed field per keyword, so the schema can be read without the
//! raw `Value` it came from.
//! The keywords it leaves out are grouped in [`SchemaKeyword`]: relationships, which
//! consensus checks against more than the value, resolution and storage keywords, and
//! annotations. Neither are
//! the rules a document type declares beside its schema, such as `propertyConstraints`,
//! which document validation checks in a step of its own.
//!
//! Document storage refuses some values the validation schema accepts: when the contract
//! uses sized integer types, an integer outside the width the document type parser derives
//! from `minimum`, `maximum` and `enum`; and wherever the parser stores an integer, a float
//! or one beyond that width (above `i64::MAX` for an unbounded one). The validation schema
//! does not model these widths.
//!
//! It follows JSON Schema, not the document type parser, where the two differ:
//!
//! * a node with a `$ref` keeps its own keywords as well as the reference, since JSON
//!   Schema applies both, where the parser reads the referenced schema alone;
//! * `required` is kept per object, as the validator compiles it, where the parser derives
//!   nested requiredness from dotted names and drops a name no property carries. At the
//!   root, the base schema has already removed the system times and heights (`$createdAt`,
//!   `$updatedAtBlockHeight`, ...), which a document's properties never carry, unless a
//!   repeated `required` key shadows the one the base schema edits;
//! * the root object's own keywords (`required`, `minProperties`, `dependentRequired`, ...)
//!   are kept, where the parser has no property to put them on.
//!
//! Every schema a `$ref` reaches is kept once, under the reference, so references between
//! definitions, cyclic ones included, stay references. The builder holds only references
//! that are JSON pointers into the document schema (`#` or `#/...`), the only ones
//! registration admits, and refuses the `$id`s that could make the JSON Schema validator
//! resolve one elsewhere: so a reference reaches the schema at its pointer.
//!
//! From protocol version 15, document validation checks every document against its type's
//! validation schema (`validate_document` 2), and refuses the documents of a type whose
//! schema has none. It is built with the `validation-schema` feature, which `validation`
//! enables, and does not need the JSON Schema validator it mirrors. Its builder is
//! generation 0 of `validation_schema`, which protocol version 15 selects: it models the
//! document validation of protocol version 14, whose parser reads `maxBytes` and
//! `encryptedFor`, and refuses a protocol version that selects no generation.

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::EncryptedFor;
use crate::data_contract::document_type::property_names::{
    ITEMS, POSITION, PROPERTIES, REF, REQUIRED,
};
use crate::data_contract::document_type::schema::last_value;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::util::json_schema::resolve_uri;
use crate::ProtocolError;
use once_cell::sync::Lazy;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
use url::Url;

mod keyword;
#[cfg(test)]
mod tests;
mod v0;
mod validate;

pub use keyword::{
    AnnotationKeyword, AnyKeyword, ArrayKeyword, CheckedBy, FormatKeyword, NumberKeyword,
    ObjectKeyword, RelationshipKeyword, ResolutionKeyword, SchemaKeyword, StorageKeyword,
    StringKeyword,
};
pub use validate::DocumentSchemaValidator;

/// A JSON Schema simple type.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    strum::EnumIter,
    strum::EnumString,
    strum::IntoStaticStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum JsonType {
    Null,
    Boolean,
    Object,
    Array,
    Number,
    String,
    Integer,
}

impl JsonType {
    /// The type's name as a schema writes it.
    pub fn name(self) -> &'static str {
        self.into()
    }

    fn from_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }
}

/// A format the JSON Schema validator knows under draft 2020-12, the pinned fork's. It
/// compiles no schema naming another (it does not ignore unknown formats), so a `$ref`
/// reaching one refuses every value.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    strum::EnumIter,
    strum::EnumString,
    strum::IntoStaticStr,
)]
pub enum ValidatorFormat {
    #[strum(serialize = "date")]
    Date,
    #[strum(serialize = "date-time")]
    DateTime,
    #[strum(serialize = "email")]
    Email,
    #[strum(serialize = "hostname")]
    Hostname,
    #[strum(serialize = "idn-email")]
    IdnEmail,
    #[strum(serialize = "ipv4")]
    Ipv4,
    #[strum(serialize = "ipv6")]
    Ipv6,
    #[strum(serialize = "regex")]
    Regex,
    #[strum(serialize = "time")]
    Time,
    #[strum(serialize = "uri")]
    Uri,
}

impl ValidatorFormat {
    /// The format's name as a schema writes it.
    pub fn name(self) -> &'static str {
        self.into()
    }
}

/// The value of the `type` keyword, in the form it was written: one JSON type, or a list
/// of them, any of which a value may be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonTypes {
    One(JsonType),
    AnyOf(Vec<JsonType>),
}

/// One schema node: the keywords it carries that constrain a value on its own
/// ([`SchemaKeyword::constrains_a_value`]), each `None` (`byteArray`: `false`) when it does
/// not.
///
/// The keywords that apply to a value of any type are held inline. The others are held by
/// group, one boxed struct per [`SchemaKeyword`] group, allocated only when the node
/// carries a keyword of the group, so a node pays only for the groups it uses. Numbers that
/// bound a value (`minimum`, `multipleOf`, ...), and `enum` and `const` values, are kept as
/// the JSON Schema validator reads them: converted to JSON as the document validator
/// converts the schema, so an integer is 64-bit and a float stays a float, bytes and
/// identifiers are arrays of integers, and a repeated map key keeps its last value. The
/// validator compares the bounds exactly; [`SchemaNode::enum_values`],
/// [`SchemaNode::constant`], [`ArrayConstraints::unique_items`] and
/// [`NumberConstraints::multiple_of`] say how it compares the others.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SchemaNode {
    /// `$ref`, as written: a JSON pointer into the document schema, `#` or `#/...`, which the
    /// validator resolves to the schema at that pointer. The key of
    /// [`ValidationSchema::references`] that holds the schema it reaches.
    pub reference: Option<String>,
    /// `type`.
    pub types: Option<JsonTypes>,
    /// `enum`, in declared order. The validator compares a value with each member exactly:
    /// numbers by value whatever their JSON type (`1` equals `1.0`), and objects key by key
    /// in the order the keys are written, so `{"b": 2, "a": 1}` is not `{"a": 1, "b": 2}`.
    pub enum_values: Option<Vec<Value>>,
    /// `const`. The validator compares a number with a number `const` in f64, within an
    /// absolute `f64::EPSILON`: `9007199254740993` matches `9007199254740992`, and `0`
    /// matches `1e-300`. It compares any other value as it compares an `enum` member.
    pub constant: Option<Value>,
    /// The [`ObjectKeyword`]s.
    pub object: Option<Box<ObjectConstraints>>,
    /// The [`ArrayKeyword`]s.
    pub array: Option<Box<ArrayConstraints>>,
    /// The [`StringKeyword`]s.
    pub string: Option<Box<StringConstraints>>,
    /// The [`NumberKeyword`]s.
    pub number: Option<Box<NumberConstraints>>,
    /// The [`FormatKeyword`]s.
    pub format: Option<Box<FormatConstraints>>,
}

/// The [`ObjectKeyword`]s of a node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectConstraints {
    /// `properties`, in declared order, each name once.
    pub properties: Option<Vec<(String, SchemaNode)>>,
    /// `additionalProperties`. Every document meta-schema holds it to `false` on the
    /// document schema and the property schemas it restricts; a system property's schema
    /// and a `contains` schema may hold `true`.
    pub additional_properties: Option<bool>,
    /// `required`, as the validator compiles it, including any name no property carries. At
    /// the root, without the system times and heights the base schema removes, unless a
    /// repeated `required` key shadows the one it edits.
    pub required: Option<Vec<String>>,
    /// `minProperties`.
    pub min_properties: Option<u64>,
    /// `maxProperties`.
    pub max_properties: Option<u64>,
    /// `dependentRequired`: for each property name, the names an object holding it must
    /// hold too.
    pub dependent_required: Option<BTreeMap<String, Vec<String>>>,
}

/// The [`ArrayKeyword`]s of a node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArrayConstraints {
    /// `items`: the schema of every element.
    pub items: Option<Box<SchemaNode>>,
    /// `contains`: a schema at least one element must match. The document meta-schemas take
    /// `contains` from JSON Schema's applicator vocabulary, which admits any schema: a
    /// boolean, or composition keywords such as `not` or `anyOf`. The builder holds a
    /// `contains` schema made of [`SchemaKeyword`]s and keys the validator ignores, and
    /// refuses any other. Nothing inside it carries `maxBytes`, `encryptedFor` or
    /// `byteArray`, which constrain nothing there.
    pub contains: Option<Box<SchemaNode>>,
    /// `minItems`.
    pub min_items: Option<u64>,
    /// `maxItems`.
    pub max_items: Option<u64>,
    /// `uniqueItems`. The validator compares the elements of an array of up to 15 by value,
    /// numbers whatever their JSON type, and those of a longer one by their JSON form: `[1,
    /// 1.0]` repeats an element, but `1` and `1.0` among 16 elements do not.
    pub unique_items: Option<bool>,
    /// `byteArray`, Platform's keyword for an array of bytes: whether the node carries it.
    /// The validator refuses any value that is not an array of integers from 0 to 255,
    /// whatever `type` says. It checks a byte array whatever value is written, so the
    /// builder refuses `false`. Never inside `contains`.
    pub byte_array: bool,
}

/// The [`StringKeyword`]s of a node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StringConstraints {
    /// `minLength`.
    pub min_length: Option<u64>,
    /// `maxLength`.
    pub max_length: Option<u64>,
    /// `maxBytes`, held only where the document type parser reads it: on a schema it reads
    /// as a property, and on the items of one, the elements of a typed array.
    pub max_bytes: Option<u64>,
    /// `pattern`, as written. The validator rewrites the ECMA escapes `\d`, `\w`, `\s`, their
    /// negations and `\cX` before compiling it with the `regex` crate under that crate's
    /// default size limit; the builder refuses a pattern that does not compile so, since the
    /// validator then refuses every value reaching it.
    pub pattern: Option<String>,
}

/// The [`NumberKeyword`]s of a node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberConstraints {
    /// `minimum`.
    pub minimum: Option<Value>,
    /// `maximum`.
    pub maximum: Option<Value>,
    /// `exclusiveMinimum`.
    pub exclusive_minimum: Option<Value>,
    /// `exclusiveMaximum`.
    pub exclusive_maximum: Option<Value>,
    /// `multipleOf`. The validator works in f64, so an integer beyond 2^53 is rounded on
    /// either side. Under an integral divisor it accepts a value with no fractional part
    /// whose remainder by the divisor is 0. Under a fractional one it accepts a value whose
    /// quotient by the divisor has a remainder modulo 1 below `f64::EPSILON`, a one-sided test
    /// every negative quotient passes: `-0.3` is accepted under `multipleOf: 0.5`. When the
    /// quotient overflows to an infinity, it decides exactly instead, on decimal readings of
    /// the value and the divisor.
    pub multiple_of: Option<Value>,
}

/// The [`FormatKeyword`]s of a node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormatConstraints {
    /// `format`.
    pub format: Option<ValidatorFormat>,
    /// `encryptedFor`, held only on a schema the document type parser reads as a property.
    pub encrypted_for: Option<EncryptedFor>,
}

/// A document type's schema as the document validator reads it: the root node and every
/// schema a `$ref` reaches.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationSchema {
    /// The document type's own schema. Its keys that are not a [`SchemaKeyword`] are the
    /// document type's other keywords, `$schema` and `$defs`, which the validation schema
    /// does not hold: none bounds a value on its own, though some, such as
    /// `propertyConstraints`, are rules document validation checks in a step of its own.
    pub root: SchemaNode,
    /// Every schema a `$ref` of the root or of another referenced schema reaches, by the
    /// reference as written.
    pub references: BTreeMap<String, Arc<SchemaNode>>,
}

impl ValidationSchema {
    /// Builds the validation schema of a schema as the document validator compiles it: a
    /// document type's schema with the contract's `$defs` and the base schema added and its
    /// property type shorthands expanded.
    ///
    /// Refuses a schema that does not convert to the JSON the validator compiles, a value it
    /// ignores or an entry a repeated key shadows included; a keyword value of the wrong
    /// shape; a key the JSON Schema validator compiles there that is not a [`SchemaKeyword`];
    /// at the root, the shapes the validator's check of
    /// the document schema against its meta-schema refuses; a `format`, `pattern` or
    /// `dependentRequired` it does not compile; `byteArray: false`; `maxBytes`, `encryptedFor`
    /// or `byteArray` where consensus does not check it (see the module documentation); a
    /// property, or `required`, repeated where the document type parser reads another entry
    /// than the validator; a document schema `$id`, and any other `$id` that is not a plain
    /// name (see [`ResolutionKeyword::Id`]); a `$ref` that is not a JSON pointer into the document
    /// schema, or that does not resolve there; and a property's `$ref` the document type
    /// parser does not resolve, or resolves to another schema.
    ///
    /// This is generation 0 of the builder ([`Self::try_from_root_v0`]), with no version
    /// check: consensus builds through [`Self::build`].
    pub(crate) fn try_from_root(root: &Value) -> Result<Self, DataContractError> {
        Self::try_from_root_v0(root).map(|(schema, _)| schema)
    }

    /// The validation schema of `root` and the patterns it compiled, built by the generation
    /// of the builder `platform_version` selects (`validation_schema`). Refuses a protocol
    /// version that selects none, whose document validation reads no validation schema, and
    /// a generation this module does not know.
    pub(crate) fn build(
        root: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, BTreeMap<String, regex::Regex>), ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .validation_schema
        {
            Some(0) => Ok(Self::try_from_root_v0(root)?),
            None => Err(ProtocolError::NotSupported(format!(
                "protocol version {} checks no document against a validation schema",
                platform_version.protocol_version
            ))),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "ValidationSchema::build".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl DataContract {
    /// The validation schema of one of the contract's document types, built from the
    /// schema the document validator compiles for it by the builder `platform_version`
    /// selects. Refuses a protocol version that selects none (`validation_schema`).
    pub fn document_type_validation_schema(
        &self,
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<ValidationSchema, ProtocolError> {
        let root = self.document_type_validated_schema(document_type_name, platform_version)?;
        let (schema, _) = ValidationSchema::build(&root, platform_version)?;
        Ok(schema)
    }

    /// The native validator of one of the contract's document types: its validation schema
    /// ([`Self::document_type_validation_schema`]) with the patterns the builder compiled.
    pub(crate) fn document_type_native_validator(
        &self,
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentSchemaValidator, ProtocolError> {
        let root = self.document_type_validated_schema(document_type_name, platform_version)?;
        DocumentSchemaValidator::try_from_root(&root, platform_version)
    }

    /// The schema the document validator compiles for one of the contract's document types:
    /// its own schema with the contract's `$defs` and the base schema added and its property
    /// type shorthands expanded.
    fn document_type_validated_schema(
        &self,
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<Value, ProtocolError> {
        let document_type = self.document_type_for_name(document_type_name)?;
        let root = DocumentType::enrich_with_base_schema(
            document_type.schema().clone(),
            self.schema_defs().map(|defs| Value::from(defs.clone())),
            platform_version,
        )?;
        Ok(
            DocumentType::expand_property_type_shorthands(&root, false, platform_version)?
                .unwrap_or(root),
        )
    }
}

impl DocumentSchemaValidator {
    /// The native validator of the document schema `root`, as the document validator of
    /// `platform_version` compiles it, built by the builder that version selects
    /// (`validation_schema`). Refuses a protocol version that selects none.
    pub(crate) fn try_from_root(
        root: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let (schema, patterns) = ValidationSchema::build(root, platform_version)?;
        Ok(Self::with_patterns(schema, patterns)?)
    }
}

impl SchemaNode {
    /// The node as a schema: the keywords it carries, in the order of
    /// [`SchemaKeyword::iter`].
    pub fn to_value(&self) -> Value {
        Value::Map(
            SchemaKeyword::iter()
                .filter_map(|keyword| {
                    self.value_of(keyword)
                        .map(|value| (Value::Text(keyword.name().into()), value))
                })
                .collect(),
        )
    }

    /// The value the node carries for a keyword, as a schema writes it: `None` when it does
    /// not carry the keyword, and for a keyword that constrains no value on its own.
    pub fn value_of(&self, keyword: SchemaKeyword) -> Option<Value> {
        match keyword {
            SchemaKeyword::Any(keyword) => match keyword {
                AnyKeyword::Ref => self.reference.clone().map(Value::Text),
                AnyKeyword::Type => self.types.as_ref().map(|types| match types {
                    JsonTypes::One(json_type) => Value::Text(json_type.name().into()),
                    JsonTypes::AnyOf(json_types) => Value::Array(
                        json_types
                            .iter()
                            .map(|json_type| Value::Text(json_type.name().into()))
                            .collect(),
                    ),
                }),
                AnyKeyword::Enum => self.enum_values.clone().map(Value::Array),
                AnyKeyword::Const => self.constant.clone(),
            },
            SchemaKeyword::Object(keyword) => {
                let object = self.object.as_deref()?;
                match keyword {
                    ObjectKeyword::Properties => object.properties.as_ref().map(|properties| {
                        Value::Map(
                            properties
                                .iter()
                                .map(|(name, node)| (Value::Text(name.clone()), node.to_value()))
                                .collect(),
                        )
                    }),
                    ObjectKeyword::AdditionalProperties => {
                        object.additional_properties.map(Value::Bool)
                    }
                    ObjectKeyword::Required => object.required.as_deref().map(texts),
                    ObjectKeyword::MinProperties => object.min_properties.map(Value::U64),
                    ObjectKeyword::MaxProperties => object.max_properties.map(Value::U64),
                    ObjectKeyword::DependentRequired => {
                        object
                            .dependent_required
                            .as_ref()
                            .map(|dependent_required| {
                                Value::Map(
                                    dependent_required
                                        .iter()
                                        .map(|(name, names)| {
                                            (Value::Text(name.clone()), texts(names))
                                        })
                                        .collect(),
                                )
                            })
                    }
                }
            }
            SchemaKeyword::Array(keyword) => {
                let array = self.array.as_deref()?;
                match keyword {
                    ArrayKeyword::Items => array.items.as_ref().map(|items| items.to_value()),
                    ArrayKeyword::Contains => {
                        array.contains.as_ref().map(|contains| contains.to_value())
                    }
                    ArrayKeyword::MinItems => array.min_items.map(Value::U64),
                    ArrayKeyword::MaxItems => array.max_items.map(Value::U64),
                    ArrayKeyword::UniqueItems => array.unique_items.map(Value::Bool),
                    ArrayKeyword::ByteArray => array.byte_array.then_some(Value::Bool(true)),
                }
            }
            SchemaKeyword::String(keyword) => {
                let string = self.string.as_deref()?;
                match keyword {
                    StringKeyword::MinLength => string.min_length.map(Value::U64),
                    StringKeyword::MaxLength => string.max_length.map(Value::U64),
                    StringKeyword::MaxBytes => string.max_bytes.map(Value::U64),
                    StringKeyword::Pattern => string.pattern.clone().map(Value::Text),
                }
            }
            SchemaKeyword::Number(keyword) => {
                let number = self.number.as_deref()?;
                match keyword {
                    NumberKeyword::Minimum => number.minimum.clone(),
                    NumberKeyword::Maximum => number.maximum.clone(),
                    NumberKeyword::ExclusiveMinimum => number.exclusive_minimum.clone(),
                    NumberKeyword::ExclusiveMaximum => number.exclusive_maximum.clone(),
                    NumberKeyword::MultipleOf => number.multiple_of.clone(),
                }
            }
            SchemaKeyword::Format(keyword) => {
                let format = self.format.as_deref()?;
                match keyword {
                    FormatKeyword::Format => format
                        .format
                        .map(|format| Value::Text(format.name().to_string())),
                    FormatKeyword::EncryptedFor => {
                        format.encrypted_for.as_ref().map(encrypted_for_value)
                    }
                }
            }
            SchemaKeyword::Relationship(_)
            | SchemaKeyword::Resolution(_)
            | SchemaKeyword::Storage(_)
            | SchemaKeyword::Annotation(_) => None,
        }
    }
}

fn encrypted_for_value(encrypted_for: &EncryptedFor) -> Value {
    Value::Map(vec![
        (
            Value::Text("recipient".into()),
            Value::Text(encrypted_for.recipient.to_string()),
        ),
        (
            Value::Text("recipientKey".into()),
            Value::Text(encrypted_for.recipient_key.clone()),
        ),
        (
            Value::Text("senderKey".into()),
            Value::Text(encrypted_for.sender_key.clone()),
        ),
        (
            Value::Text("scheme".into()),
            Value::Text(encrypted_for.scheme.as_str().into()),
        ),
    ])
}

fn texts(names: &[String]) -> Value {
    Value::Array(names.iter().cloned().map(Value::Text).collect())
}

fn structure_error(message: String) -> DataContractError {
    DataContractError::InvalidContractStructure(message)
}

/// The keywords the JSON Schema validator compiles under draft 2020-12 besides Platform's
/// `byteArray`: the pinned fork's keyword table, with `then` and `else`, which `if` reads,
/// and `minContains` and `maxContains`, which `contains` reads. It compiles some of them
/// only beside another ([`v0::compiled_there`]) and no other key, though before compiling it
/// checks the document schema itself against the draft 2020-12 meta-schema (see
/// [`v0::refuse_what_the_meta_schema_refuses_at_the_root`]).
const VALIDATOR_KEYWORDS: [&str; 40] = [
    "$ref",
    "additionalItems",
    "additionalProperties",
    "allOf",
    "anyOf",
    "const",
    "contains",
    "dependencies",
    "dependentRequired",
    "dependentSchemas",
    "else",
    "enum",
    "exclusiveMaximum",
    "exclusiveMinimum",
    "format",
    "if",
    "items",
    "maxContains",
    "maxItems",
    "maxLength",
    "maxProperties",
    "maximum",
    "minContains",
    "minItems",
    "minLength",
    "minProperties",
    "minimum",
    "multipleOf",
    "not",
    "oneOf",
    "pattern",
    "patternProperties",
    "prefixItems",
    "properties",
    "propertyNames",
    "required",
    "then",
    "type",
    "unevaluatedProperties",
    "uniqueItems",
];

/// The ECMA control escape `\cX`, which the pinned fork's `convert_regex` replaces first.
static CONTROL_ESCAPE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"\\c[A-Za-z]").expect("a literal pattern compiles"));

/// `pattern` compiled as the JSON Schema validator compiles it, `None` when it does not
/// compile. The pinned fork's `convert_regex` replaces each ECMA control escape `\cX` with its
/// control character, rewrites the classes `\d`, `\D`, `\w`, `\W`, `\s` and `\S` as explicit
/// ones, and compiles the result with the `regex` crate, which refuses look-around and
/// backreferences. The size limit is the crate's default: the document validator compiles
/// a contract it loaded from state with default `regex` options, and every node validates
/// documents against a contract so loaded. Registration compiles the schema under a smaller
/// limit, so every pattern the document schema reaches directly also compiles here; one a
/// `$ref` target holds is compiled only when a document reaches it, under the default.
fn validator_regex(pattern: &str) -> Option<regex::Regex> {
    let controlled = CONTROL_ESCAPE.replace_all(pattern, |captures: &regex::Captures| {
        // The escape's letter, an ASCII one, uppercased is at least `A` (65)
        captures
            .get(0)
            .and_then(|escape| escape.as_str().as_bytes().get(2))
            .and_then(|letter| letter.to_ascii_uppercase().checked_sub(64))
            .map(|control| char::from(control).to_string())
            .unwrap_or_default()
    });
    let mut converted = String::with_capacity(controlled.len());
    let mut chars = controlled.chars();
    while let Some(current) = chars.next() {
        if current != '\\' {
            converted.push(current);
            continue;
        }
        match chars.next() {
            Some('d') => converted.push_str("[0-9]"),
            Some('D') => converted.push_str("[^0-9]"),
            Some('w') => converted.push_str("[A-Za-z0-9_]"),
            Some('W') => converted.push_str("[^A-Za-z0-9_]"),
            Some('s') => {
                converted.push_str("[ \t\n\r\u{000b}\u{000c}\u{2003}\u{feff}\u{2029}\u{00a0}]")
            }
            Some('S') => {
                converted.push_str("[^ \t\n\r\u{000b}\u{000c}\u{2003}\u{feff}\u{2029}\u{00a0}]")
            }
            Some(next) => {
                converted.push(current);
                converted.push(next);
            }
            None => converted.push(current),
        }
    }
    regex::Regex::new(&converted).ok()
}
