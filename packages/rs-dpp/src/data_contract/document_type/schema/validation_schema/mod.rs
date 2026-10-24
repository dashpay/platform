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
//! schema has none. It is built with the `validation-schema` feature, which `validation` enables, and does not need the JSON Schema validator it
//! mirrors. It models the document validation of protocol version 14 and later, whose parser
//! reads `maxBytes` and `encryptedFor`, and refuses an earlier protocol version.

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
    pub(crate) fn try_from_root(root: &Value) -> Result<Self, DataContractError> {
        Self::try_from_root_with_patterns(root).map(|(schema, _)| schema)
    }

    /// [`Self::try_from_root`], with every `pattern` the schema holds compiled as the
    /// builder compiled it to check it, for the native validator to reuse.
    pub(crate) fn try_from_root_with_patterns(
        root: &Value,
    ) -> Result<(Self, BTreeMap<String, regex::Regex>), DataContractError> {
        // The document validator converts the whole schema before it compiles it, so a value
        // JSON cannot hold anywhere, even one the validator would then ignore or drop as a
        // shadowed entry of a repeated key, leaves it nothing to compile
        root.try_to_validating_json().map_err(|error| {
            structure_error(format!(
                "the document schema does not convert to the JSON the JSON Schema validator \
                 compiles: {error}"
            ))
        })?;
        let mut reached = Reached::default();
        let root_node = build_node(root, Reading::Document, &mut reached)?;
        refuse_identifiers_that_may_move_a_reference(root)?;
        let mut references = BTreeMap::new();
        // Each schema a reference reaches is built once in each reading that reaches it, for
        // the keywords that reading refuses: the readings build the same node, which every
        // reference reaching the schema shares. Building only queues references, so each
        // (schema, reading) pair is built once and the queue runs dry.
        let mut built = BTreeSet::new();
        let mut nodes: BTreeMap<*const Value, Arc<SchemaNode>> = BTreeMap::new();
        while let Some((reference, reading)) = reached.pending.pop_front() {
            let target = resolve_reference(root, &reference)?;
            if reading == Reading::ReferencedProperty {
                refuse_a_property_reference_the_parser_resolves_elsewhere(
                    root, &reference, target,
                )?;
            }
            let address = std::ptr::from_ref(target);
            if built.insert((address, reading)) {
                let node = build_node(target, reading, &mut reached)
                    .map_err(|error| reached_through(error, &reference))?;
                nodes.entry(address).or_insert_with(|| Arc::new(node));
            }
            if let Some(node) = nodes.get(&address) {
                references
                    .entry(reference)
                    .or_insert_with(|| Arc::clone(node));
            }
        }
        Ok((
            Self {
                root: root_node,
                references,
            },
            reached.patterns,
        ))
    }
}

/// `error`, raised building the schema `reference` reaches, saying so.
fn reached_through(error: DataContractError, reference: &str) -> DataContractError {
    match error {
        DataContractError::InvalidContractStructure(message) => {
            structure_error(format!("{message} (reached through `$ref` `{reference}`)"))
        }
        error => error,
    }
}

impl DataContract {
    /// The validation schema of one of the contract's document types, built from the
    /// schema the document validator compiles for it. Refuses a protocol version before 14,
    /// whose document validation checks neither `maxBytes` nor `encryptedFor`, which the
    /// validation schema holds where the parser of protocol version 14 reads them.
    pub fn document_type_validation_schema(
        &self,
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<ValidationSchema, ProtocolError> {
        let root = self.document_type_validated_schema(document_type_name, platform_version)?;
        Ok(ValidationSchema::try_from_root(&root)?)
    }

    /// The native validator of one of the contract's document types: its validation schema
    /// ([`Self::document_type_validation_schema`]) with the patterns the builder compiled.
    pub(crate) fn document_type_native_validator(
        &self,
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentSchemaValidator, ProtocolError> {
        let root = self.document_type_validated_schema(document_type_name, platform_version)?;
        let (schema, patterns) = ValidationSchema::try_from_root_with_patterns(&root)?;
        Ok(DocumentSchemaValidator::with_patterns(schema, patterns)?)
    }

    /// The schema the document validator compiles for one of the contract's document types:
    /// its own schema with the contract's `$defs` and the base schema added and its property
    /// type shorthands expanded.
    fn document_type_validated_schema(
        &self,
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<Value, ProtocolError> {
        if platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .validate_max_bytes
            .is_none()
        {
            return Err(ProtocolError::NotSupported(format!(
                "the validation schema models the document validation of protocol version 14 \
                 and later, not of protocol version {}",
                platform_version.protocol_version
            )));
        }
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
/// only beside another ([`compiled_there`]) and no other key, though before compiling it
/// checks the document schema itself against the draft 2020-12 meta-schema (see
/// [`refuse_what_the_meta_schema_refuses_at_the_root`]).
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

/// A plain-name anchor, as the draft 2020-12 meta-schema's `anchorString` admits one.
static ANCHOR: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"^[A-Za-z_][-A-Za-z0-9._]*$").expect("a literal pattern compiles")
});

/// A URI reference, as the JSON Schema validator's `uri-reference` format admits one when it
/// checks the document schema against its meta-schema.
static URI_REFERENCE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"^(\w+:(/?/?))?[^#\\\s]*(#[^\\\s]*)?\z").expect("a literal pattern compiles")
});

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

/// What building a schema reaches besides its nodes.
#[derive(Default)]
struct Reached {
    /// Every `$ref` met, with the reading of the schema it reaches, to build.
    pending: VecDeque<(String, Reading)>,
    /// Every `pattern` met, compiled ([`validator_regex`]).
    patterns: BTreeMap<String, regex::Regex>,
}

impl Reached {
    /// Whether the JSON Schema validator compiles `pattern`, keeping it compiled.
    fn compile_pattern(&mut self, pattern: &str) -> bool {
        if self.patterns.contains_key(pattern) {
            return true;
        }
        match validator_regex(pattern) {
            Some(regex) => {
                self.patterns.insert(pattern.to_string(), regex);
                true
            }
            None => false,
        }
    }
}

/// How consensus reads a schema node, for the three keywords the module documentation says
/// consensus checks only in some places: `maxBytes`, `encryptedFor` and `byteArray`. The
/// builder refuses each keyword where consensus does not check it, so every one the
/// validation schema holds bounds the value where it is held.
///
/// The parser read is protocol version 14's: every `properties` entry of the document
/// schema and, below it, of an object property is a property; a property's `$ref` is
/// resolved once, and the referenced schema read in the property's place; the `items` of a
/// property are the element schema of a typed array; and `contains` is never read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Reading {
    /// The document schema: the parser reads its `properties` as properties.
    Document,
    /// A `properties` entry the parser reads as a property. With a `$ref`, it reads the
    /// referenced schema in its place, and neither the entry's own keywords nor the
    /// schemas below them.
    Property,
    /// The schema a property's `$ref` reaches, which the parser reads as the property, as
    /// it is: it does not follow the schema's own `$ref`.
    ReferencedProperty,
    /// The `items` of a property, which the parser reads as the element schema of a typed
    /// array, for its `maxBytes`.
    Items,
    /// A schema only the JSON Schema validator reads.
    Schema,
    /// A schema inside `contains`.
    Contains,
}

impl Reading {
    /// Whether the document type parser reads a node read this way as a property,
    /// `has_reference` when the node carries a `$ref`: a property without one, or the
    /// schema a property's `$ref` reaches.
    fn parsed_as_property(self, has_reference: bool) -> bool {
        match self {
            Reading::Property => !has_reference,
            Reading::ReferencedProperty => true,
            _ => false,
        }
    }

    /// Whether consensus checks `keyword` on a node read this way, `has_reference` when the
    /// node carries a `$ref`.
    fn checks(self, keyword: SchemaKeyword, has_reference: bool) -> bool {
        match keyword {
            SchemaKeyword::String(StringKeyword::MaxBytes) if self == Reading::Items => true,
            keyword if keyword.checked_by() == CheckedBy::DocumentTypeParser => {
                self.parsed_as_property(has_reference)
            }
            SchemaKeyword::Array(ArrayKeyword::ByteArray) => self != Reading::Contains,
            _ => true,
        }
    }

    /// How consensus reads the schema `keyword` holds on a node read this way,
    /// `has_reference` when the node carries a `$ref`.
    fn below(self, keyword: SchemaKeyword, has_reference: bool) -> Reading {
        let parsed_as_property = self.parsed_as_property(has_reference);
        match keyword {
            _ if self == Reading::Contains => Reading::Contains,
            SchemaKeyword::Array(ArrayKeyword::Contains) => Reading::Contains,
            SchemaKeyword::Object(ObjectKeyword::Properties)
                if parsed_as_property || self == Reading::Document =>
            {
                Reading::Property
            }
            SchemaKeyword::Array(ArrayKeyword::Items) if parsed_as_property => Reading::Items,
            SchemaKeyword::Any(AnyKeyword::Ref) if self == Reading::Property => {
                Reading::ReferencedProperty
            }
            _ => Reading::Schema,
        }
    }
}

/// Refuses a property's `$ref` that the document type parser resolves to another schema
/// than `target`, the one the JSON Schema validator reaches, whose `maxBytes` and
/// `encryptedFor` the parser would read as the property's, or does not resolve at all, so
/// that it builds no document type.
fn refuse_a_property_reference_the_parser_resolves_elsewhere(
    root: &Value,
    reference: &str,
    target: &Value,
) -> Result<(), DataContractError> {
    match resolve_uri(root, reference) {
        Ok(parsed) if std::ptr::eq(parsed, target) => Ok(()),
        Ok(_) => Err(structure_error(format!(
            "the document type parser resolves `$ref` `{reference}` to another schema than \
             the JSON Schema validator does"
        ))),
        Err(_) => Err(structure_error(format!(
            "the document type parser does not resolve `$ref` `{reference}`, which the JSON \
             Schema validator resolves"
        ))),
    }
}

/// Refuses the `$id`s that may make the JSON Schema validator resolve a `$ref` elsewhere
/// than the schema at its pointer, where the builder resolves it. The validator resolves a
/// `$ref` against the URL the `$id`s of the schemas above it set, looks it up first among
/// the schemas an `$id` anywhere names, and fails the whole schema on an `$id` it cannot
/// join. Without a document schema `$id`, a plain name, made of ASCII letters, digits, `_`,
/// `-` and `.`, does none of this: it resolves to a `json-schema:` URL, which the validator
/// reads as no `$id` at all, and it names no fragment a `$ref` could reach. Such names occur
/// in the data of Platform keywords the validator's resolver walks as it walks a schema,
/// such as a `refersTo` `findBy`, so any other `$id` is refused wherever it is written.
fn refuse_identifiers_that_may_move_a_reference(root: &Value) -> Result<(), DataContractError> {
    if let Some(id) = root
        .as_map()
        .and_then(|map| {
            map.iter()
                .find(|(key, value)| key.as_text() == Some("$id") && value.is_text())
        })
        .and_then(|(_, id)| id.as_text())
    {
        return Err(structure_error(format!(
            "the document schema carries `$id` `{id}`, against which the JSON Schema validator \
             resolves every `$ref` (or compiles nothing, for a relative one), which the \
             validation schema does not represent"
        )));
    }
    refuse_identifiers_that_are_not_plain_names(root)
}

/// Refuses an `$id` written as text that is not a plain name, anywhere in `value`.
fn refuse_identifiers_that_are_not_plain_names(value: &Value) -> Result<(), DataContractError> {
    match value {
        Value::Map(map) => map.iter().try_for_each(|(key, value)| {
            if let (Some("$id"), Some(id)) = (key.as_text(), value.as_text()) {
                let plain = !id.is_empty()
                    && id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
                if !plain {
                    return Err(structure_error(format!(
                        "`$id` `{id}` is not a plain name (ASCII letters, digits, `_`, `-` and \
                         `.`), so the JSON Schema validator may resolve a `$ref` through it or \
                         against it, which the validation schema does not represent"
                    )));
                }
            }
            refuse_identifiers_that_are_not_plain_names(value)
        }),
        Value::Array(elements) => elements
            .iter()
            .try_for_each(refuse_identifiers_that_are_not_plain_names),
        _ => Ok(()),
    }
}

/// The entries of a schema map as the JSON Schema validator reads them: a map converts to
/// a JSON object with its order preserved, so a repeated key keeps the place of its first
/// entry and takes the value of its last.
fn last_wins(map: &[(Value, Value)]) -> Result<Vec<(&str, &Value)>, DataContractError> {
    let mut entries: Vec<(&str, &Value)> = Vec::with_capacity(map.len());
    for (key, value) in map {
        let key = key
            .as_text()
            .ok_or_else(|| structure_error("schema keys must be strings".to_string()))?;
        match entries.iter_mut().find(|(seen, _)| *seen == key) {
            Some(entry) => entry.1 = value,
            None => entries.push((key, value)),
        }
    }
    Ok(entries)
}

/// Whether `schema`, read by the document type parser as a property, carries where the
/// parser reads it a keyword it reads into a property, or a `$ref`, through which it may
/// reach one: on the schema itself, and down its `properties` and `items`, the only schemas
/// below a property the parser reads. Data it never reads, such as `examples`, `enum` and
/// `const` values or `contains`, is not scanned.
fn carries_parser_keywords(schema: &Value) -> bool {
    schema.as_map().is_some_and(|map| {
        map.iter().any(|(key, value)| match key.as_text() {
            Some(PROPERTIES) => properties_carry_parser_keywords(value),
            Some(ITEMS) => match value {
                Value::Array(elements) => elements.iter().any(carries_parser_keywords),
                value => carries_parser_keywords(value),
            },
            key => key
                .and_then(SchemaKeyword::from_name)
                .is_some_and(|keyword| {
                    keyword.checked_by() == CheckedBy::DocumentTypeParser
                        || keyword == SchemaKeyword::Any(AnyKeyword::Ref)
                }),
        })
    })
}

/// Whether a property of `properties` [carries parser keywords](carries_parser_keywords).
fn properties_carry_parser_keywords(properties: &Value) -> bool {
    properties.as_map().is_some_and(|properties| {
        properties
            .iter()
            .any(|(_, schema)| carries_parser_keywords(schema))
    })
}

/// Refuses a key `map` repeats, `only` that key when given, when an entry of it carries
/// `maxBytes`, `encryptedFor` or a `$ref` where the document type parser reads it
/// ([`carries_parser_keywords`]; of a repeated `properties`, in one of its properties) and the
/// document type
/// parser reads another of its entries than the JSON Schema validator, which reads the last:
/// the bounds the parser checks would not be those of the entry the validation schema holds.
/// The parser reads the first entry of a property name repeated below the document schema,
/// and of a repeated `properties` of the document schema; of a name the document schema's
/// own `properties` repeat, `by_position`, it reads the entry with the highest `position`.
fn refuse_repeated_entries_the_parser_reads_otherwise(
    map: &[(Value, Value)],
    only: Option<&str>,
    by_position: bool,
) -> Result<(), DataContractError> {
    let mut entries: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for (key, value) in map {
        if let Some(key) = key.as_text() {
            if only.is_none_or(|only| only == key) {
                entries.entry(key).or_default().push(value);
            }
        }
    }
    for (key, values) in entries {
        if values.len() > 1
            && values.iter().any(|value| {
                if only == Some(PROPERTIES) {
                    properties_carry_parser_keywords(value)
                } else {
                    carries_parser_keywords(value)
                }
            })
            && !(by_position && the_last_has_the_highest_position(&values))
        {
            return Err(structure_error(format!(
                "`{key}` is declared more than once with `maxBytes`, `encryptedFor` or a `$ref`, \
                 and the document type parser reads another of its entries than the JSON \
                 Schema validator does"
            )));
        }
    }
    Ok(())
}

/// Whether the document type parser keeps the last of the entries of a name the document
/// schema's own `properties` repeat: it orders those properties by `position`, keeping the
/// order of equal ones, and keeps the last entry of a name, so the entry with the highest
/// position, the last of those on a tie.
fn the_last_has_the_highest_position(entries: &[&Value]) -> bool {
    let position = |entry: &Value| {
        entry
            .as_map()
            .and_then(|map| map.iter().find(|(key, _)| key.as_text() == Some(POSITION)))
            .and_then(|(_, position)| position.as_integer::<u64>())
    };
    let Some(last) = entries.last().and_then(|entry| position(entry)) else {
        return false;
    };
    entries
        .iter()
        .all(|entry| position(entry).is_some_and(|position| position <= last))
}

/// Refuses a `required` the map repeats, when its first entry names other properties than its
/// last. The document type parser requires of a document the names of the first entry of
/// each `required` it finds walking `properties` down from the document schema, where the
/// JSON Schema validator reads the last. The builder does not model that walk, which follows
/// no `$ref` and reads `required` beside one, so it refuses such a `required` wherever it is
/// written.
fn refuse_a_required_repeated_differently(map: &[(Value, Value)]) -> Result<(), DataContractError> {
    // The parser reads the text entries of a list and nothing else
    fn names(entry: &Value) -> BTreeSet<&str> {
        entry
            .as_array()
            .map(|list| list.iter().filter_map(Value::as_text).collect())
            .unwrap_or_default()
    }
    let mut entries = map
        .iter()
        .filter(|(key, _)| key.as_text() == Some(REQUIRED))
        .map(|(_, entry)| entry);
    if let (Some(first), Some(last)) = (entries.next(), entries.next_back()) {
        if names(first) != names(last) {
            return Err(structure_error(
                "`required` is declared more than once, its first entry naming other \
                 properties than its last, and the document type parser may require those of \
                 its first entry where the JSON Schema validator reads the last"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

/// Whether `entries` names one entry twice.
fn repeats_an_entry<'e>(entries: impl IntoIterator<Item = &'e str>) -> bool {
    let mut seen = BTreeSet::new();
    entries.into_iter().any(|entry| !seen.insert(entry))
}

/// Whether the JSON Schema validator compiles the keyword `name`, valued `value`, of a schema
/// whose entries are `entries`: `then` and `else` only beside `if`, `if` only beside one of
/// them, the bounds of `contains` only beside it, and `additionalItems` only beside an
/// `items` list (when itself a schema object or `false`) or `items: false`; any other keyword
/// of [`VALIDATOR_KEYWORDS`] wherever it is written.
fn compiled_there(name: &str, value: &Value, entries: &[(&str, &Value)]) -> bool {
    let beside = |other: &str| {
        entries
            .iter()
            .find(|(key, _)| *key == other)
            .map(|(_, entry)| *entry)
    };
    match name {
        "then" | "else" => beside("if").is_some(),
        "if" => beside("then").is_some() || beside("else").is_some(),
        "minContains" | "maxContains" => beside("contains").is_some(),
        "additionalItems" => match beside("items") {
            Some(Value::Array(_)) => matches!(value, Value::Map(_) | Value::Bool(false)),
            Some(Value::Bool(false)) => true,
            _ => false,
        },
        _ => VALIDATOR_KEYWORDS.contains(&name),
    }
}

fn text<'a>(value: &'a Value, keyword: &str) -> Result<&'a str, DataContractError> {
    value
        .as_text()
        .ok_or_else(|| structure_error(format!("`{keyword}` must be a string")))
}

fn texts_of(value: &Value, keyword: &str) -> Result<Vec<String>, DataContractError> {
    // As the validator reads it: bytes are an array of integers
    as_validated(value, keyword)?
        .as_array()
        .ok_or_else(|| structure_error(format!("`{keyword}` must be an array of strings")))?
        .iter()
        .map(|element| text(element, keyword).map(str::to_string))
        .collect()
}

fn boolean(value: &Value, keyword: &str) -> Result<bool, DataContractError> {
    value
        .as_bool()
        .ok_or_else(|| structure_error(format!("`{keyword}` must be a boolean")))
}

fn count(value: &Value, keyword: &str) -> Result<u64, DataContractError> {
    value
        .as_integer::<u64>()
        .ok_or_else(|| structure_error(format!("`{keyword}` must be a non-negative integer")))
}

/// A keyword value as the JSON Schema validator reads it: converted to JSON as the document
/// validator converts the schema, so every integer is 64-bit, bytes and identifiers are
/// arrays of integers and a repeated map key keeps its last value, and back. Refuses what
/// the conversion refuses: a non-finite float, an integer beyond 64 bits, a key that is not
/// a string.
fn as_validated(value: &Value, keyword: &str) -> Result<Value, DataContractError> {
    value
        .try_to_validating_json()
        .map(Value::from)
        .map_err(|error| structure_error(format!("`{keyword}` is not a JSON value: {error}")))
}

fn number(value: &Value, keyword: &str) -> Result<Value, DataContractError> {
    let value = as_validated(value, keyword)?;
    if value.is_integer() || value.is_float() {
        Ok(value)
    } else {
        Err(structure_error(format!("`{keyword}` must be a number")))
    }
}

fn types(value: &Value) -> Result<JsonTypes, DataContractError> {
    let json_type = |name: &str| {
        JsonType::from_name(name).ok_or_else(|| structure_error(format!("unknown type `{name}`")))
    };
    match value {
        Value::Text(name) => Ok(JsonTypes::One(json_type(name)?)),
        Value::Array(names) => Ok(JsonTypes::AnyOf(
            names
                .iter()
                .map(|name| json_type(text(name, "type")?))
                .collect::<Result<_, _>>()?,
        )),
        _ => Err(structure_error(
            "`type` must be a string or an array of strings".to_string(),
        )),
    }
}

/// Refuses a document schema the JSON Schema validator compiles nothing for. Before it
/// compiles, the validator checks the document schema against the draft 2020-12
/// meta-schema, under draft 7 rules (the meta-schema's `$schema` lacks the trailing `#` the
/// validator recognises it by): formats are validated, the keywords beside a `$ref` are
/// dropped, and no schema below the document schema is checked (the meta-schema reaches it
/// through `$dynamicRef`, which the validator does not implement). Of what that check
/// refuses, these are the root keys the builder would otherwise hold or skip, read as the
/// validator reads them, converted to JSON: a `required` or `type` list naming an entry
/// twice; an empty `type` list; a `multipleOf` not above 0; a `minContains` or `maxContains`
/// that is not a non-negative integer; an annotation or content keyword of the wrong type;
/// a `definitions` or `$defs` that is not an object; a `$vocabulary` that is not an object
/// of booleans by URI; a `$schema` that is not a URI; an anchor that is not a plain name; and
/// an `$id`, `$ref`, `$dynamicRef` or `$recursiveRef` the validator's `uri-reference` format
/// refuses. Its `format: regex` check of a root `pattern`, made with another regex engine
/// than the one that compiles patterns, is not repeated. Registration refuses each of them.
fn refuse_what_the_meta_schema_refuses_at_the_root(
    entries: &[(&str, &Value)],
) -> Result<(), DataContractError> {
    for (name, value) in entries {
        let admitted: fn(&Value) -> bool = match *name {
            "required" => |value| {
                value
                    .as_array()
                    .is_none_or(|list| !repeats_an_entry(list.iter().filter_map(Value::as_text)))
            },
            "type" => |value| {
                value.as_array().is_none_or(|list| {
                    !list.is_empty() && !repeats_an_entry(list.iter().filter_map(Value::as_text))
                })
            },
            "multipleOf" => |value| value.to_float().is_ok_and(|divisor| divisor > 0.0),
            "minContains" | "maxContains" => |value| {
                value
                    .to_float()
                    .is_ok_and(|bound| bound >= 0.0 && bound.fract() == 0.0)
            },
            "title" | "description" | "$comment" | "contentMediaType" | "contentEncoding" => {
                Value::is_text
            }
            "deprecated" | "readOnly" | "writeOnly" => Value::is_bool,
            "examples" => Value::is_array,
            "definitions" | "$defs" => Value::is_map,
            "$vocabulary" => |value| {
                value.as_map().is_some_and(|map| {
                    map.iter().all(|(key, entry)| {
                        key.as_text().is_some_and(|uri| Url::parse(uri).is_ok()) && entry.is_bool()
                    })
                })
            },
            "$schema" => |value| value.as_text().is_some_and(|uri| Url::parse(uri).is_ok()),
            "$anchor" | "$dynamicAnchor" | "$recursiveAnchor" => |value| {
                value
                    .as_text()
                    .is_some_and(|anchor| ANCHOR.is_match(anchor))
            },
            "$id" | "$ref" | "$dynamicRef" | "$recursiveRef" => |value| {
                value
                    .as_text()
                    .is_some_and(|uri| URI_REFERENCE.is_match(uri))
            },
            _ => continue,
        };
        if !admitted(&as_validated(value, name)?) {
            return Err(structure_error(format!(
                "the JSON Schema validator's draft 2020-12 meta-schema refuses the document \
                 schema's `{name}`, so it compiles nothing for it"
            )));
        }
    }
    Ok(())
}

/// Builds the node of `schema`, read by consensus as `reading`, and queues every `$ref` it
/// holds, with the reading of the schema the reference reaches. A key that is not a
/// [`SchemaKeyword`] is skipped, but one the JSON Schema validator compiles is refused.
fn build_node(
    schema: &Value,
    reading: Reading,
    reached: &mut Reached,
) -> Result<SchemaNode, DataContractError> {
    let map = schema
        .as_map()
        .ok_or_else(|| structure_error("a schema must be an object".to_string()))?;
    if reading == Reading::Document {
        refuse_repeated_entries_the_parser_reads_otherwise(map, Some(PROPERTIES), false)?;
    }
    refuse_a_required_repeated_differently(map)?;
    let entries = last_wins(map)?;
    if reading == Reading::Document {
        refuse_what_the_meta_schema_refuses_at_the_root(&entries)?;
    }
    let has_reference = entries.iter().any(|(name, _)| *name == REF);
    let mut node = SchemaNode::default();
    for &(name, value) in &entries {
        let Some(keyword) = SchemaKeyword::from_name(name) else {
            if compiled_there(name, value, &entries) {
                return Err(structure_error(format!(
                    "the JSON Schema validator checks `{name}`, which the validation schema \
                     does not hold"
                )));
            }
            continue;
        };
        if !reading.checks(keyword, has_reference) {
            let place = match reading {
                Reading::Contains => "inside `contains`, where it constrains nothing",
                Reading::Property => {
                    "beside a property's `$ref`, whose referenced schema the document type \
                     parser reads in its place"
                }
                _ => "where the document type parser does not read it",
            };
            return Err(structure_error(format!(
                "`{name}` is read {place}, so consensus does not check it there"
            )));
        }
        let below = reading.below(keyword, has_reference);
        match keyword {
            SchemaKeyword::Any(keyword) => match keyword {
                AnyKeyword::Ref => {
                    let reference = text(value, name)?.to_string();
                    reached.pending.push_back((reference.clone(), below));
                    node.reference = Some(reference);
                }
                AnyKeyword::Type => node.types = Some(types(value)?),
                AnyKeyword::Enum => {
                    let Value::Array(values) = as_validated(value, name)? else {
                        return Err(structure_error("`enum` must be an array".to_string()));
                    };
                    node.enum_values = Some(values);
                }
                AnyKeyword::Const => node.constant = Some(as_validated(value, name)?),
            },
            SchemaKeyword::Object(keyword) => {
                let object = node.object.get_or_insert_with(Default::default);
                match keyword {
                    ObjectKeyword::Properties => {
                        let properties_map = value.as_map().ok_or_else(|| {
                            structure_error("`properties` must be an object".to_string())
                        })?;
                        if below == Reading::Property {
                            refuse_repeated_entries_the_parser_reads_otherwise(
                                properties_map,
                                None,
                                reading == Reading::Document,
                            )?;
                        }
                        let properties = last_wins(properties_map)?
                            .into_iter()
                            .map(|(property_name, property)| {
                                Ok((
                                    property_name.to_string(),
                                    build_node(property, below, reached)?,
                                ))
                            })
                            .collect::<Result<_, DataContractError>>()?;
                        object.properties = Some(properties);
                    }
                    ObjectKeyword::AdditionalProperties => {
                        object.additional_properties = Some(boolean(value, name)?)
                    }
                    ObjectKeyword::Required => object.required = Some(texts_of(value, name)?),
                    ObjectKeyword::MinProperties => {
                        object.min_properties = Some(count(value, name)?)
                    }
                    ObjectKeyword::MaxProperties => {
                        object.max_properties = Some(count(value, name)?)
                    }
                    ObjectKeyword::DependentRequired => {
                        let dependent_required = last_wins(value.as_map().ok_or_else(|| {
                            structure_error("`dependentRequired` must be an object".to_string())
                        })?)?
                        .into_iter()
                        .map(|(property_name, names)| {
                            let names = texts_of(names, name)?;
                            if repeats_an_entry(names.iter().map(String::as_str)) {
                                return Err(structure_error(format!(
                                    "`{name}` lists a name twice for `{property_name}`, and the \
                                     JSON Schema validator compiles no schema that does"
                                )));
                            }
                            Ok((property_name.to_string(), names))
                        })
                        .collect::<Result<_, DataContractError>>()?;
                        object.dependent_required = Some(dependent_required);
                    }
                }
            }
            SchemaKeyword::Array(keyword) => {
                let array = node.array.get_or_insert_with(Default::default);
                match keyword {
                    ArrayKeyword::Items => {
                        array.items = Some(Box::new(build_node(value, below, reached)?))
                    }
                    ArrayKeyword::Contains => {
                        array.contains = Some(Box::new(build_node(value, below, reached)?))
                    }
                    ArrayKeyword::MinItems => array.min_items = Some(count(value, name)?),
                    ArrayKeyword::MaxItems => array.max_items = Some(count(value, name)?),
                    ArrayKeyword::UniqueItems => array.unique_items = Some(boolean(value, name)?),
                    ArrayKeyword::ByteArray => {
                        if !boolean(value, name)? {
                            return Err(structure_error(
                                "`byteArray: false` is checked as `true`: the JSON Schema \
                                 validator checks a byte array whatever value is written"
                                    .to_string(),
                            ));
                        }
                        array.byte_array = true
                    }
                }
            }
            SchemaKeyword::String(keyword) => {
                let string = node.string.get_or_insert_with(Default::default);
                match keyword {
                    StringKeyword::MinLength => string.min_length = Some(count(value, name)?),
                    StringKeyword::MaxLength => string.max_length = Some(count(value, name)?),
                    StringKeyword::MaxBytes => string.max_bytes = Some(count(value, name)?),
                    StringKeyword::Pattern => {
                        let pattern = text(value, name)?;
                        if !reached.compile_pattern(pattern) {
                            return Err(structure_error(format!(
                                "the JSON Schema validator compiles no `{name}` `{pattern}`, so \
                                 it refuses every value reaching it"
                            )));
                        }
                        string.pattern = Some(pattern.to_string())
                    }
                }
            }
            SchemaKeyword::Number(keyword) => {
                let bound = Some(number(value, name)?);
                let number = node.number.get_or_insert_with(Default::default);
                match keyword {
                    NumberKeyword::Minimum => number.minimum = bound,
                    NumberKeyword::Maximum => number.maximum = bound,
                    NumberKeyword::ExclusiveMinimum => number.exclusive_minimum = bound,
                    NumberKeyword::ExclusiveMaximum => number.exclusive_maximum = bound,
                    NumberKeyword::MultipleOf => number.multiple_of = bound,
                }
            }
            SchemaKeyword::Format(keyword) => {
                let format = node.format.get_or_insert_with(Default::default);
                match keyword {
                    FormatKeyword::Format => {
                        let format_name = text(value, name)?;
                        let Ok(validator_format) = format_name.parse::<ValidatorFormat>() else {
                            return Err(structure_error(format!(
                                "the JSON Schema validator knows no format `{format_name}` and \
                                 compiles no schema that names it"
                            )));
                        };
                        format.format = Some(validator_format)
                    }
                    FormatKeyword::EncryptedFor => {
                        format.encrypted_for = Some(
                            platform_value::from_value(as_validated(value, name)?).map_err(
                                |error| structure_error(format!("`{name}` is malformed: {error}")),
                            )?,
                        )
                    }
                }
            }
            SchemaKeyword::Relationship(_)
            | SchemaKeyword::Resolution(_)
            | SchemaKeyword::Storage(_)
            | SchemaKeyword::Annotation(_) => {}
        }
    }
    Ok(node)
}

/// Resolves a `$ref` to the schema at its JSON pointer in the document schema, where the
/// JSON Schema validator reaches it once [`refuse_identifiers_that_may_move_a_reference`] has
/// passed. Refuses a reference that is not `#` or `#/...`, or that holds `%`, whitespace or a
/// control character: the validator parses a reference into a URL, which drops or encodes
/// those, and percent-decodes its fragment, so only without them is the pointer it follows
/// the one written. In each segment `~1` is `/` and `~0` is `~`, an array index has no sign
/// or leading zero, and a segment naming a repeated map key reaches its last entry, as
/// [`last_wins`] reads the map.
fn resolve_reference<'a>(root: &'a Value, reference: &str) -> Result<&'a Value, DataContractError> {
    let pointer = reference
        .strip_prefix('#')
        .filter(|pointer| pointer.is_empty() || pointer.starts_with('/'))
        .filter(|pointer| {
            !pointer.contains(|c: char| c == '%' || c.is_whitespace() || c.is_control())
        })
        .ok_or_else(|| {
            structure_error(format!(
                "`$ref` `{reference}` is not a JSON pointer into the document schema (`#` or \
                 `#/...`, without `%`, whitespace or control characters), which the validation \
                 schema does not represent"
            ))
        })?;
    let unresolved = || {
        structure_error(format!(
            "`$ref` `{reference}` does not resolve within the document schema"
        ))
    };
    let mut target = root;
    let Some(pointer) = pointer.strip_prefix('/') else {
        return Ok(target);
    };
    for segment in pointer.split('/') {
        let segment = if segment.contains('~') {
            Cow::Owned(segment.replace("~1", "/").replace("~0", "~"))
        } else {
            Cow::Borrowed(segment)
        };
        target = match target {
            Value::Map(map) => last_value(map, &segment).ok_or_else(unresolved)?,
            Value::Array(elements) => array_index(&segment)
                .and_then(|index| elements.get(index))
                .ok_or_else(unresolved)?,
            _ => return Err(unresolved()),
        };
    }
    Ok(target)
}

/// An array index in a JSON pointer segment, as the JSON Schema validator reads one: decimal
/// digits with no sign and no leading zero.
fn array_index(segment: &str) -> Option<usize> {
    if segment.starts_with('+') || (segment.starts_with('0') && segment.len() != 1) {
        None
    } else {
        segment.parse().ok()
    }
}
