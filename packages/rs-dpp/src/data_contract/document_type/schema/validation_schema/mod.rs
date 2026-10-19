//! A document type's JSON schema as the document validator reads it, typed.
//!
//! The document validator compiles a document type's schema with the contract's `$defs`
//! and the base schema added and its property type shorthands expanded, and checks every
//! document value against it with JSON Schema (draft 2020-12) semantics. Consensus also
//! checks two keywords of that schema that bound one value but that JSON Schema leaves
//! alone: `maxBytes` on a string, which the document validator checks right after, and
//! `encryptedFor` on a byte array, which the validation of a create or replace checks.
//! Both are read from the property the document type parser builds, so only where the
//! parser reads a schema as a property: not beside a property's `$ref`, whose referenced
//! schema the parser reads in its place, and not inside `contains`. Platform's `byteArray`
//! keyword, which the JSON Schema validator checks, constrains nothing inside `contains`
//! either. The builder refuses each of these three keywords where consensus does not check
//! it.
//! [`ValidationSchema`] holds exactly the keywords of that schema that constrain a value on
//! its own, both kinds, one typed field per keyword, so the schema can be read without the
//! raw `Value` it came from.
//! The keywords it leaves out are grouped in [`SchemaKeyword`]: relationships, which
//! consensus checks against more than the value, meta keywords and annotations.
//!
//! It follows JSON Schema, not the document type parser, where the two differ:
//!
//! * a node with a `$ref` keeps its own keywords as well as the reference, since JSON
//!   Schema applies both, where the parser reads the referenced schema alone;
//! * `required` is kept per object, as the validator compiles it, where the parser derives
//!   nested requiredness from dotted names and drops a name no property carries. At the
//!   root, the base schema has already removed the system times and heights (`$createdAt`,
//!   `$updatedAtBlockHeight`, ...), which a document's properties never carry;
//! * the root object's own keywords (`required`, `minProperties`, `dependentRequired`, ...)
//!   are kept, where the parser has no property to put them on.
//!
//! Every schema a `$ref` reaches is built once and kept by the reference as written, so
//! references between definitions, cyclic ones included, stay references. A reference is
//! resolved as the JSON Schema validator resolves it, from the URL it parses the reference
//! into: through the schemas an `$id` names first, then by its JSON pointer.
//!
//! Nothing reads it yet at any protocol version. It is built with the `validation-schema`
//! feature, which `validation` enables, and does not need the JSON Schema validator it
//! mirrors, only the `url` and `percent-encoding` crates that validator resolves references
//! with.

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::EncryptedFor;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::util::json_schema::resolve_uri;
use crate::ProtocolError;
use percent_encoding::percent_decode_str;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use url::Url;

mod keyword;
#[cfg(test)]
mod tests;

pub use keyword::{
    AnnotationKeyword, AnyKeyword, ArrayKeyword, FormatKeyword, MetaKeyword, NumberKeyword,
    ObjectKeyword, RelationshipKeyword, SchemaKeyword, StringKeyword,
};

/// A JSON Schema simple type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
        match self {
            JsonType::Null => "null",
            JsonType::Boolean => "boolean",
            JsonType::Object => "object",
            JsonType::Array => "array",
            JsonType::Number => "number",
            JsonType::String => "string",
            JsonType::Integer => "integer",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "null" => JsonType::Null,
            "boolean" => JsonType::Boolean,
            "object" => JsonType::Object,
            "array" => JsonType::Array,
            "number" => JsonType::Number,
            "string" => JsonType::String,
            "integer" => JsonType::Integer,
            _ => return None,
        })
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
/// ([`SchemaKeyword::constrains_a_value`]), each `None` when it does not.
///
/// The keywords that apply to a value of any type are held inline. The others are held by
/// group, one boxed struct per [`SchemaKeyword`] group, allocated only when the node
/// carries a keyword of the group, so a node pays only for the groups it uses. Numbers that
/// bound a value (`minimum`, `multipleOf`, ...), and `enum` and `const` values, are kept as
/// the JSON Schema validator compares them: converted to JSON as the document validator
/// converts the schema, so an integer is 64-bit and a float stays a float, bytes and
/// identifiers are arrays of integers, and a repeated map key keeps its last value.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SchemaNode {
    /// `$ref`: a JSON pointer into the schema the document type compiles, a key of
    /// [`ValidationSchema::references`].
    pub reference: Option<String>,
    /// `type`.
    pub types: Option<JsonTypes>,
    /// `enum`, in declared order.
    pub enum_values: Option<Vec<Value>>,
    /// `const`.
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
    /// `properties`, in declared order.
    pub properties: Option<Vec<(String, SchemaNode)>>,
    /// `additionalProperties`. Every document meta-schema holds it to `false`.
    pub additional_properties: Option<bool>,
    /// `required`, as the validator compiles it, including any name no property carries. At
    /// the root, without the system times and heights the base schema removes.
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
    /// `contains` schema made of [`SchemaKeyword`]s only and refuses any other; no contract
    /// on mainnet or testnet declares `contains` at all. Nothing inside it carries
    /// `maxBytes`, `encryptedFor` or `byteArray`, which constrain nothing there.
    pub contains: Option<Box<SchemaNode>>,
    /// `minItems`.
    pub min_items: Option<u64>,
    /// `maxItems`.
    pub max_items: Option<u64>,
    /// `uniqueItems`.
    pub unique_items: Option<bool>,
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
    /// `pattern`.
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
    /// `multipleOf`.
    pub multiple_of: Option<Value>,
}

/// The [`FormatKeyword`]s of a node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormatConstraints {
    /// `format`.
    pub format: Option<String>,
    /// `byteArray`, Platform's keyword for an array of bytes, with the value it was
    /// written with. Never inside `contains`.
    pub byte_array: Option<bool>,
    /// `encryptedFor`, held only on a schema the document type parser reads as a property.
    pub encrypted_for: Option<EncryptedFor>,
}

/// A document type's schema as the document validator reads it: the root node and every
/// schema a `$ref` reaches.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ValidationSchema {
    /// The document type's own schema. Its keys that are not a [`SchemaKeyword`] are the
    /// document type's other keywords, `$schema` and `$defs`, which constrain no value.
    pub root: SchemaNode,
    /// Every schema a `$ref` of the root or of another referenced schema reaches, by the
    /// reference as written.
    pub references: BTreeMap<String, SchemaNode>,
}

impl ValidationSchema {
    /// Builds the validation schema of a schema as the document validator compiles it: a
    /// document type's schema with the contract's `$defs` and the base schema added and its
    /// property type shorthands expanded.
    ///
    /// Refuses a keyword value of the wrong shape; below the root, a key that is not a
    /// [`SchemaKeyword`], and in the schema of a system property and at the root, one the
    /// JSON Schema validator compiles; a `format` it does not know; `maxBytes`,
    /// `encryptedFor` or `byteArray` where consensus does not check it (see the module
    /// documentation); a `$ref` that does not resolve within the schema; a property's `$ref`
    /// the document type parser resolves to another schema; and a `$ref` an `$id` above it
    /// rescopes (see [`MetaKeyword::Id`]).
    pub fn try_from_root(root: &Value) -> Result<Self, DataContractError> {
        let root_url = document_url(root)?;
        let mut reached = Reached::default();
        let root_node = build_node(root, OtherKeys::Document, Reading::Document, &mut reached)?;
        let mut references = BTreeMap::new();
        if reached.pending.is_empty() {
            return Ok(Self {
                root: root_node,
                references,
            });
        }
        let identifiers = Identifiers::of(root)?;
        // A schema reached in several readings is built in each, for the keywords each
        // refuses, and kept once: the readings build the same node.
        let mut visited = BTreeSet::new();
        while let Some((reference, reading)) = reached.pending.pop_front() {
            if !visited.insert((reference.clone(), reading)) {
                continue;
            }
            let target = resolve_reference(root, &root_url, &identifiers.named, &reference)?;
            if reading == Reading::ReferencedProperty {
                refuse_a_property_reference_the_parser_resolves_elsewhere(
                    root, &reference, target,
                )?;
            }
            // The root reached by a reference keeps the document type keywords beside its
            // own, which a nested schema would refuse. It is built again in the reference's
            // reading only for the keywords that reading refuses (a property's `maxBytes`
            // when `contains` reaches the root, say): the node built is the root's.
            let other_keys = if std::ptr::eq(target, root) {
                OtherKeys::Document
            } else {
                OtherKeys::Refused
            };
            let node = build_node(target, other_keys, reading, &mut reached)?;
            references.entry(reference).or_insert(node);
        }
        refuse_references_an_id_rescopes(&identifiers, &root_url, &reached.schemas)?;
        Ok(Self {
            root: root_node,
            references,
        })
    }
}

impl DataContract {
    /// The validation schema of one of the contract's document types, built from the
    /// schema the document validator compiles for it.
    pub fn document_type_validation_schema(
        &self,
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<ValidationSchema, ProtocolError> {
        let document_type = self.document_type_for_name(document_type_name)?;
        let root = DocumentType::enrich_with_base_schema(
            document_type.schema().clone(),
            self.schema_defs().map(|defs| Value::from(defs.clone())),
            platform_version,
        )?;
        let root = DocumentType::expand_property_type_shorthands(&root, false, platform_version)?
            .unwrap_or(root);
        Ok(ValidationSchema::try_from_root(&root)?)
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
                    FormatKeyword::Format => format.format.clone().map(Value::Text),
                    FormatKeyword::ByteArray => format.byte_array.map(Value::Bool),
                    FormatKeyword::EncryptedFor => {
                        format.encrypted_for.as_ref().map(encrypted_for_value)
                    }
                }
            }
            SchemaKeyword::Relationship(_)
            | SchemaKeyword::Meta(_)
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

/// The URL the JSON Schema validator resolves a schema against when no `$id` sets
/// another.
const DEFAULT_ROOT_URL: &str = "json-schema:///";

/// The scheme of [`DEFAULT_ROOT_URL`]. An `$id` resolving to a URL of this scheme leaves
/// the JSON Schema validator compiling the schemas below it against the default URL.
const DEFAULT_ROOT_SCHEME: &str = "json-schema";

/// The keywords the JSON Schema validator compiles under draft 2020-12 besides Platform's
/// `byteArray`: the pinned fork's keyword table, with `then` and `else`, which `if` reads,
/// and `minContains` and `maxContains`, which `contains` reads. It ignores any other key.
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

/// The formats the JSON Schema validator knows under draft 2020-12, the pinned fork's. It
/// compiles no schema naming another (it does not ignore unknown formats), so a `$ref`
/// reaching one refuses every value.
const VALIDATOR_FORMATS: [&str; 10] = [
    "date",
    "date-time",
    "email",
    "hostname",
    "idn-email",
    "ipv4",
    "ipv6",
    "regex",
    "time",
    "uri",
];

/// What a schema node may carry besides the [`SchemaKeyword`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OtherKeys {
    /// The document schema's: keys the JSON Schema validator ignores, the document type's
    /// own keywords, `$schema` and `$defs`, which constrain no value.
    Document,
    /// Keys the JSON Schema validator ignores: in the schema of a system property
    /// (`$createdAt`, ...), which every document meta-schema admits as any schema.
    Ignored,
    /// None: in a schema whose keys a document meta-schema restricts to the keywords it
    /// admits (`documentSchema`, `documentArrayItem`), so another key is one the validation
    /// schema would miss.
    Refused,
}

impl OtherKeys {
    /// What the schema of the property `name` of a node carrying `self` may carry: a
    /// system property's, named by the only property names starting with `$` that a
    /// document meta-schema admits, any key the validator ignores.
    fn of_property(self, name: &str) -> OtherKeys {
        match self {
            OtherKeys::Document if name.starts_with('$') => OtherKeys::Ignored,
            OtherKeys::Document => OtherKeys::Refused,
            other_keys => other_keys,
        }
    }

    /// What the schemas below a node carrying `self`, but its properties', may carry.
    fn below(self) -> OtherKeys {
        match self {
            OtherKeys::Document => OtherKeys::Refused,
            other_keys => other_keys,
        }
    }
}

/// What building a schema reaches besides its nodes.
#[derive(Default)]
struct Reached {
    /// Every `$ref` met, with the reading of the schema it reaches, to build.
    pending: VecDeque<(String, Reading)>,
    /// Every schema built, by address: the schemas the validator compiles, whose references
    /// the `$id` check reads, and no copy of one elsewhere.
    schemas: BTreeSet<*const Value>,
}

fn default_url() -> Result<Url, DataContractError> {
    Url::parse(DEFAULT_ROOT_URL)
        .map_err(|error| structure_error(format!("invalid default schema URL: {error}")))
}

/// The URL the JSON Schema validator resolves the document schema's references against:
/// the root's `$id`, which must be an absolute URL for the validator to compile the schema
/// at all, but a `json-schema:` one, which leaves it on [`DEFAULT_ROOT_URL`], as no `$id`
/// does.
fn document_url(root: &Value) -> Result<Url, DataContractError> {
    let default_url = default_url()?;
    let Some(map) = root.as_map() else {
        return Ok(default_url);
    };
    let Some(id) = last_wins(map)?
        .into_iter()
        .find(|(key, _)| *key == "$id")
        .and_then(|(_, id)| id.as_text())
    else {
        return Ok(default_url);
    };
    let url = Url::parse(id).map_err(|error| {
        structure_error(format!(
            "the document schema's `$id` `{id}` is not an absolute URL, so the JSON Schema \
             validator compiles no schema from it: {error}"
        ))
    })?;
    Ok(if url.scheme() == DEFAULT_ROOT_SCHEME {
        default_url
    } else {
        url
    })
}

/// How consensus reads a schema node, for the three keywords of a document schema that the
/// JSON Schema validator does not check on its own: `maxBytes` and `encryptedFor`, which the
/// document type parser reads into the properties it builds, and `byteArray`, Platform's
/// custom keyword. The validator checks `byteArray` where it reports errors, but inside
/// `contains` it only asks whether an element matches, and `byteArray` says every element
/// does. The builder refuses each keyword where consensus does not check it, so every one
/// the validation schema holds bounds the value where it is held.
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
    /// Whether consensus checks `keyword` on a node read this way, `has_reference` when the
    /// node carries a `$ref`.
    fn checks(self, keyword: SchemaKeyword, has_reference: bool) -> bool {
        let parsed_as_property = match self {
            Reading::Property => !has_reference,
            Reading::ReferencedProperty => true,
            _ => false,
        };
        match keyword {
            SchemaKeyword::String(StringKeyword::MaxBytes) => {
                parsed_as_property || self == Reading::Items
            }
            SchemaKeyword::Format(FormatKeyword::EncryptedFor) => parsed_as_property,
            SchemaKeyword::Format(FormatKeyword::ByteArray) => self != Reading::Contains,
            _ => true,
        }
    }

    /// How consensus reads the schema `keyword` holds on a node read this way,
    /// `has_reference` when the node carries a `$ref`.
    fn below(self, keyword: SchemaKeyword, has_reference: bool) -> Reading {
        let parsed = match self {
            Reading::Document | Reading::ReferencedProperty => true,
            Reading::Property => !has_reference,
            Reading::Items | Reading::Schema | Reading::Contains => false,
        };
        match keyword {
            _ if self == Reading::Contains => Reading::Contains,
            SchemaKeyword::Array(ArrayKeyword::Contains) => Reading::Contains,
            SchemaKeyword::Object(ObjectKeyword::Properties) if parsed => Reading::Property,
            SchemaKeyword::Array(ArrayKeyword::Items) if parsed && self != Reading::Document => {
                Reading::Items
            }
            SchemaKeyword::Any(AnyKeyword::Ref) if self == Reading::Property => {
                Reading::ReferencedProperty
            }
            _ => Reading::Schema,
        }
    }
}

/// Refuses a property's `$ref` that the document type parser resolves to another schema
/// than `target`, the one the JSON Schema validator reaches: the parser reads that other
/// schema's `maxBytes` and `encryptedFor` as the property's.
fn refuse_a_property_reference_the_parser_resolves_elsewhere(
    root: &Value,
    reference: &str,
    target: &Value,
) -> Result<(), DataContractError> {
    match resolve_uri(root, reference) {
        Ok(parsed) if std::ptr::eq(parsed, target) => Ok(()),
        _ => Err(structure_error(format!(
            "the document type parser resolves `$ref` `{reference}` to another schema than \
             the JSON Schema validator does"
        ))),
    }
}

/// What the JSON Schema validator reads of a schema's identifiers.
struct Identifiers<'a> {
    /// Every schema an `$id` names, by the URL the validator's resolver indexes it at. A URL
    /// named twice names the later schema, as the resolver's index keeps the last.
    named: BTreeMap<String, &'a Value>,
    /// Every `$ref`, with the address of the schema holding it and the URL the validator
    /// compiles it with there, `None` when the reference is not a URL reference.
    references: Vec<(*const Value, &'a str, Option<Url>)>,
}

impl<'a> Identifiers<'a> {
    /// The identifiers of the document schema `root`.
    fn of(root: &'a Value) -> Result<Self, DataContractError> {
        let default_url = default_url()?;
        let mut identifiers = Identifiers {
            named: BTreeMap::new(),
            references: Vec::new(),
        };
        identifiers.collect(root, Some(&default_url), None, &default_url)?;
        Ok(identifiers)
    }

    /// Collects the identifiers of `schema` as the JSON Schema validator reads them.
    ///
    /// Its resolver indexes every map carrying an `$id` at the URL the `$id` resolves to
    /// against `named_scope`, the URL the nearest `$id` above names (an empty fragment
    /// dropped). It skips the value of every member named `enum` or `const`, a property so
    /// named included, where `named_scope` is `None`.
    ///
    /// Its compiler gives a `$ref` the URL it resolves to against `compile_scope`, the URL
    /// the nearest `$id` above resolves to, or `default_url` when there is none or that URL
    /// is a `json-schema:` one, where `compile_scope` is `None`. A relative `$id` therefore
    /// never changes the URL of a reference below it.
    fn collect(
        &mut self,
        schema: &'a Value,
        named_scope: Option<&Url>,
        compile_scope: Option<&Url>,
        default_url: &Url,
    ) -> Result<(), DataContractError> {
        match schema {
            Value::Map(map) => {
                let entries = last_wins(map)?;
                let mut named_here = None;
                let mut compiled_here = None;
                if let Some(id) = entries
                    .iter()
                    .find(|(key, _)| *key == "$id")
                    .and_then(|(_, id)| id.as_text())
                {
                    let not_a_url = |error: url::ParseError| {
                        structure_error(format!("`$id` `{id}` is not a URL reference: {error}"))
                    };
                    if let Some(scope) = named_scope {
                        let mut named = scope.join(id).map_err(not_a_url)?;
                        if named.fragment() == Some("") {
                            named.set_fragment(None);
                        }
                        self.named.insert(named.to_string(), schema);
                        named_here = Some(named);
                    }
                    compiled_here = Some(
                        compile_scope
                            .unwrap_or(default_url)
                            .join(id)
                            .map_err(not_a_url)?,
                    );
                }
                let named_scope = match named_here.as_ref() {
                    Some(named) => Some(named),
                    None => named_scope,
                };
                let compile_scope = match compiled_here.as_ref() {
                    Some(compiled) if compiled.scheme() == DEFAULT_ROOT_SCHEME => None,
                    Some(compiled) => Some(compiled),
                    None => compile_scope,
                };
                for (key, value) in entries {
                    if key == "$ref" {
                        if let Some(reference) = value.as_text() {
                            let url =
                                reference_url(compile_scope.unwrap_or(default_url), reference);
                            self.references
                                .push((std::ptr::from_ref(schema), reference, url));
                        }
                    }
                    let named_scope = match key {
                        "enum" | "const" => None,
                        _ => named_scope,
                    };
                    self.collect(value, named_scope, compile_scope, default_url)?;
                }
                Ok(())
            }
            Value::Array(elements) => elements.iter().try_for_each(|element| {
                self.collect(element, named_scope, compile_scope, default_url)
            }),
            _ => Ok(()),
        }
    }
}

/// Refuses a schema in which the JSON Schema validator gives a `$ref` the builder reached
/// another URL than [`resolve_reference`] resolves: one below an absolute `$id`, which
/// rescopes it, as the validation schema does not represent. `built` holds every schema
/// built, by address, so a copy of a reference the validator never compiles, in `examples`
/// say, is not read.
fn refuse_references_an_id_rescopes(
    identifiers: &Identifiers,
    root_url: &Url,
    built: &BTreeSet<*const Value>,
) -> Result<(), DataContractError> {
    for (schema, reference, url) in &identifiers.references {
        if built.contains(schema) && (url.is_none() || reference_url(root_url, reference) != *url) {
            return Err(structure_error(format!(
                "`$ref` `{reference}` is given another URL by an `$id` above it, which the \
                 validation schema does not represent"
            )));
        }
    }
    Ok(())
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

fn text<'a>(value: &'a Value, keyword: &str) -> Result<&'a str, DataContractError> {
    value
        .as_text()
        .ok_or_else(|| structure_error(format!("`{keyword}` must be a string")))
}

fn texts_of(value: &Value, keyword: &str) -> Result<Vec<String>, DataContractError> {
    value
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

/// Builds the node of `schema`, read by consensus as `reading` and carrying besides its
/// keywords what `other_keys` admits, and queues every `$ref` it holds, with the reading of
/// the schema the reference reaches.
fn build_node(
    schema: &Value,
    other_keys: OtherKeys,
    reading: Reading,
    reached: &mut Reached,
) -> Result<SchemaNode, DataContractError> {
    let map = schema
        .as_map()
        .ok_or_else(|| structure_error("a schema must be an object".to_string()))?;
    reached.schemas.insert(std::ptr::from_ref(schema));
    let entries = last_wins(map)?;
    let has_reference = entries.iter().any(|(name, _)| *name == "$ref");
    let mut node = SchemaNode::default();
    for (name, value) in entries {
        let Some(keyword) = SchemaKeyword::from_name(name) else {
            if other_keys == OtherKeys::Refused {
                return Err(structure_error(format!(
                    "a document schema does not support the keyword `{name}`"
                )));
            }
            if VALIDATOR_KEYWORDS.contains(&name) {
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
                "`{name}` is declared {place}, so consensus does not check it there"
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
                        let properties = last_wins(value.as_map().ok_or_else(|| {
                            structure_error("`properties` must be an object".to_string())
                        })?)?
                        .into_iter()
                        .map(|(property_name, property)| {
                            Ok((
                                property_name.to_string(),
                                build_node(
                                    property,
                                    other_keys.of_property(property_name),
                                    below,
                                    reached,
                                )?,
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
                            Ok((property_name.to_string(), texts_of(names, name)?))
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
                        array.items = Some(Box::new(build_node(
                            value,
                            other_keys.below(),
                            below,
                            reached,
                        )?))
                    }
                    ArrayKeyword::Contains => {
                        array.contains = Some(Box::new(build_node(
                            value,
                            other_keys.below(),
                            below,
                            reached,
                        )?))
                    }
                    ArrayKeyword::MinItems => array.min_items = Some(count(value, name)?),
                    ArrayKeyword::MaxItems => array.max_items = Some(count(value, name)?),
                    ArrayKeyword::UniqueItems => array.unique_items = Some(boolean(value, name)?),
                }
            }
            SchemaKeyword::String(keyword) => {
                let string = node.string.get_or_insert_with(Default::default);
                match keyword {
                    StringKeyword::MinLength => string.min_length = Some(count(value, name)?),
                    StringKeyword::MaxLength => string.max_length = Some(count(value, name)?),
                    StringKeyword::MaxBytes => string.max_bytes = Some(count(value, name)?),
                    StringKeyword::Pattern => string.pattern = Some(text(value, name)?.to_string()),
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
                        if !VALIDATOR_FORMATS.contains(&format_name) {
                            return Err(structure_error(format!(
                                "the JSON Schema validator knows no format `{format_name}` and \
                                 compiles no schema that names it"
                            )));
                        }
                        format.format = Some(format_name.to_string())
                    }
                    FormatKeyword::ByteArray => format.byte_array = Some(boolean(value, name)?),
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
            | SchemaKeyword::Meta(_)
            | SchemaKeyword::Annotation(_) => {}
        }
    }
    Ok(node)
}

/// The URL the JSON Schema validator parses a `$ref` into, against `base`: the parse drops
/// tabs and newlines anywhere in the reference and trims the spaces and control characters
/// that lead or trail it. `None` when the reference is not a URL reference.
fn reference_url(base: &Url, reference: &str) -> Option<Url> {
    base.join(reference).ok()
}

/// Resolves a `$ref` to the schema the JSON Schema validator reaches through it, from its URL
/// against the root ([`reference_url`]), short of an `$id` above the reference rescoping it,
/// which [`refuse_references_an_id_rescopes`] refuses. As the validator does, it looks the
/// URL up among the schemas an `$id` names (`named`) first, so a location-independent
/// `#label` resolves; then takes the document the URL without its fragment names, the
/// schema itself at [`DEFAULT_ROOT_URL`] whatever its `$id`, or a schema an `$id` names;
/// and follows the fragment there, percent-decoded as a whole so an encoded `/` (`%2F`)
/// separates segments, as a JSON pointer: in each segment `~1` is `/` and `~0` is `~`, an
/// array index has no sign or leading zero, and a segment naming a repeated map key reaches
/// its last entry, as [`last_wins`] reads the map.
fn resolve_reference<'a>(
    root: &'a Value,
    root_url: &Url,
    named: &BTreeMap<String, &'a Value>,
    reference: &str,
) -> Result<&'a Value, DataContractError> {
    let unresolved = || {
        structure_error(format!(
            "`$ref` `{reference}` does not resolve within the document schema"
        ))
    };
    let url = reference_url(root_url, reference).ok_or_else(unresolved)?;
    if let Some(schema) = named.get(url.as_str()) {
        return Ok(schema);
    }
    let mut document_url = url.clone();
    document_url.set_fragment(None);
    let document = if document_url.as_str() == DEFAULT_ROOT_URL {
        root
    } else {
        named
            .get(document_url.as_str())
            .copied()
            .ok_or_else(unresolved)?
    };
    let pointer = percent_decode_str(url.fragment().unwrap_or_default())
        .decode_utf8()
        .map_err(|_| unresolved())?;
    if pointer.is_empty() {
        return Ok(document);
    }
    let pointer = pointer.strip_prefix('/').ok_or_else(unresolved)?;
    let mut target = document;
    for segment in pointer.split('/') {
        let segment = segment.replace("~1", "/").replace("~0", "~");
        target = match target {
            Value::Map(map) => map
                .iter()
                .rev()
                .find(|(key, _)| key.as_text() == Some(segment.as_str()))
                .map(|(_, value)| value)
                .ok_or_else(unresolved)?,
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
