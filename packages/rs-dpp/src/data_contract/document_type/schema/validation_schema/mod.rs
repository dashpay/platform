//! A document type's JSON schema as the document validator reads it, typed.
//!
//! The document validator compiles a document type's schema with the contract's `$defs`
//! and the base schema added and its property type shorthands expanded, and checks every
//! document value against it with JSON Schema (draft 2020-12) semantics. [`ValidationSchema`]
//! holds exactly the keywords of that schema that constrain a value, one typed field per
//! keyword, so the schema can be read without the raw `Value` it came from.
//!
//! It follows JSON Schema, not the document type parser, where the two differ:
//!
//! * a node with a `$ref` keeps its own keywords as well as the reference, since JSON
//!   Schema applies both, where the parser reads the referenced schema alone;
//! * `required` is kept per object, as written, where the parser derives nested
//!   requiredness from dotted names and drops a name no property carries;
//! * the root object's own keywords (`required`, `minProperties`, `dependentRequired`, ...)
//!   are kept, where the parser has no property to put them on.
//!
//! Every schema a `$ref` reaches is built once and kept by the reference as written, so
//! references between definitions, cyclic ones included, stay references.
//!
//! Nothing reads it yet at any protocol version.

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::EncryptedFor;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, VecDeque};

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

/// The `type` keyword, in the form it was written: one type, or a list a value matches if
/// it is of any of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKeyword {
    One(JsonType),
    AnyOf(Vec<JsonType>),
}

/// One schema node: the keywords it carries that constrain a value on its own
/// ([`SchemaKeyword::constrains_a_value`]), each `None` when it does not. Numbers that bound
/// a value (`minimum`, `multipleOf`, ...), and `enum` and `const` values, are kept as the
/// schema wrote them, integer or float.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SchemaNode {
    /// `$ref`: a JSON pointer into the schema the document type compiles, a key of
    /// [`ValidationSchema::references`].
    pub reference: Option<String>,
    /// `type`.
    pub types: Option<TypeKeyword>,
    /// `enum`, in declared order.
    pub enum_values: Option<Vec<Value>>,
    /// `const`.
    pub constant: Option<Value>,
    /// `properties`, in declared order.
    pub properties: Option<Vec<(String, SchemaNode)>>,
    /// `additionalProperties`. Every document meta-schema holds it to `false`.
    pub additional_properties: Option<bool>,
    /// `required`, as written, including any name no property carries.
    pub required: Option<Vec<String>>,
    /// `minProperties`.
    pub min_properties: Option<u64>,
    /// `maxProperties`.
    pub max_properties: Option<u64>,
    /// `dependentRequired`: for each property name, the names an object holding it must
    /// hold too.
    pub dependent_required: Option<BTreeMap<String, Vec<String>>>,
    /// `items`: the schema of every element.
    pub items: Option<Box<SchemaNode>>,
    /// `contains`: a schema at least one element must match.
    pub contains: Option<Box<SchemaNode>>,
    /// `minItems`.
    pub min_items: Option<u64>,
    /// `maxItems`.
    pub max_items: Option<u64>,
    /// `uniqueItems`.
    pub unique_items: Option<bool>,
    /// `minLength`.
    pub min_length: Option<u64>,
    /// `maxLength`.
    pub max_length: Option<u64>,
    /// `maxBytes`.
    pub max_bytes: Option<u64>,
    /// `pattern`.
    pub pattern: Option<String>,
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
    /// `format`.
    pub format: Option<String>,
    /// `byteArray`, Platform's keyword for an array of bytes, with the value it was
    /// written with.
    pub byte_array: Option<bool>,
    /// `contentMediaType`.
    pub content_media_type: Option<String>,
    /// `encryptedFor`.
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
    /// Refuses a key below the root that is not a [`SchemaKeyword`], a keyword value of the
    /// wrong shape, and a `$ref` that does not resolve.
    pub fn try_from_root(root: &Value) -> Result<Self, DataContractError> {
        let mut pending = VecDeque::new();
        let root_node = build_node(root, true, &mut pending)?;
        let mut references = BTreeMap::new();
        while let Some(reference) = pending.pop_front() {
            if references.contains_key(&reference) {
                continue;
            }
            let target = resolve_pointer(root, &reference)?;
            let node = build_node(target, false, &mut pending)?;
            references.insert(reference, node);
        }
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
    /// not carry the keyword, and for a keyword that constrains no value.
    pub fn value_of(&self, keyword: SchemaKeyword) -> Option<Value> {
        match keyword {
            SchemaKeyword::Any(keyword) => match keyword {
                AnyKeyword::Ref => self.reference.clone().map(Value::Text),
                AnyKeyword::Type => self.types.as_ref().map(|types| match types {
                    TypeKeyword::One(json_type) => Value::Text(json_type.name().into()),
                    TypeKeyword::AnyOf(json_types) => Value::Array(
                        json_types
                            .iter()
                            .map(|json_type| Value::Text(json_type.name().into()))
                            .collect(),
                    ),
                }),
                AnyKeyword::Enum => self.enum_values.clone().map(Value::Array),
                AnyKeyword::Const => self.constant.clone(),
            },
            SchemaKeyword::Object(keyword) => match keyword {
                ObjectKeyword::Properties => self.properties.as_ref().map(|properties| {
                    Value::Map(
                        properties
                            .iter()
                            .map(|(name, node)| (Value::Text(name.clone()), node.to_value()))
                            .collect(),
                    )
                }),
                ObjectKeyword::AdditionalProperties => self.additional_properties.map(Value::Bool),
                ObjectKeyword::Required => self.required.as_deref().map(texts),
                ObjectKeyword::MinProperties => self.min_properties.map(Value::U64),
                ObjectKeyword::MaxProperties => self.max_properties.map(Value::U64),
                ObjectKeyword::DependentRequired => {
                    self.dependent_required.as_ref().map(|dependent_required| {
                        Value::Map(
                            dependent_required
                                .iter()
                                .map(|(name, names)| (Value::Text(name.clone()), texts(names)))
                                .collect(),
                        )
                    })
                }
            },
            SchemaKeyword::Array(keyword) => match keyword {
                ArrayKeyword::Items => self.items.as_ref().map(|items| items.to_value()),
                ArrayKeyword::Contains => {
                    self.contains.as_ref().map(|contains| contains.to_value())
                }
                ArrayKeyword::MinItems => self.min_items.map(Value::U64),
                ArrayKeyword::MaxItems => self.max_items.map(Value::U64),
                ArrayKeyword::UniqueItems => self.unique_items.map(Value::Bool),
            },
            SchemaKeyword::String(keyword) => match keyword {
                StringKeyword::MinLength => self.min_length.map(Value::U64),
                StringKeyword::MaxLength => self.max_length.map(Value::U64),
                StringKeyword::MaxBytes => self.max_bytes.map(Value::U64),
                StringKeyword::Pattern => self.pattern.clone().map(Value::Text),
            },
            SchemaKeyword::Number(keyword) => match keyword {
                NumberKeyword::Minimum => self.minimum.clone(),
                NumberKeyword::Maximum => self.maximum.clone(),
                NumberKeyword::ExclusiveMinimum => self.exclusive_minimum.clone(),
                NumberKeyword::ExclusiveMaximum => self.exclusive_maximum.clone(),
                NumberKeyword::MultipleOf => self.multiple_of.clone(),
            },
            SchemaKeyword::Format(keyword) => match keyword {
                FormatKeyword::Format => self.format.clone().map(Value::Text),
                FormatKeyword::ByteArray => self.byte_array.map(Value::Bool),
                FormatKeyword::ContentMediaType => self.content_media_type.clone().map(Value::Text),
                FormatKeyword::EncryptedFor => self.encrypted_for.as_ref().map(encrypted_for_value),
            },
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

fn number(value: &Value, keyword: &str) -> Result<Value, DataContractError> {
    if value.is_integer() || value.is_float() {
        Ok(value.clone())
    } else {
        Err(structure_error(format!("`{keyword}` must be a number")))
    }
}

fn types(value: &Value) -> Result<TypeKeyword, DataContractError> {
    let json_type = |name: &str| {
        JsonType::from_name(name).ok_or_else(|| structure_error(format!("unknown type `{name}`")))
    };
    match value {
        Value::Text(name) => Ok(TypeKeyword::One(json_type(name)?)),
        Value::Array(names) => Ok(TypeKeyword::AnyOf(
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

fn build_node(
    schema: &Value,
    is_root: bool,
    pending: &mut VecDeque<String>,
) -> Result<SchemaNode, DataContractError> {
    let map = schema
        .as_map()
        .ok_or_else(|| structure_error("a schema must be an object".to_string()))?;
    let mut node = SchemaNode::default();
    for (key, value) in map {
        let name = key
            .as_text()
            .ok_or_else(|| structure_error("schema keys must be strings".to_string()))?;
        let Some(keyword) = SchemaKeyword::from_name(name) else {
            // The root also carries the document type's own keywords, `$schema` and `$defs`.
            if is_root {
                continue;
            }
            return Err(structure_error(format!(
                "a document schema does not support the keyword `{name}`"
            )));
        };
        match keyword {
            SchemaKeyword::Any(keyword) => match keyword {
                AnyKeyword::Ref => {
                    let reference = text(value, name)?.to_string();
                    pending.push_back(reference.clone());
                    node.reference = Some(reference);
                }
                AnyKeyword::Type => node.types = Some(types(value)?),
                AnyKeyword::Enum => {
                    node.enum_values = Some(
                        value
                            .as_array()
                            .ok_or_else(|| structure_error("`enum` must be an array".to_string()))?
                            .clone(),
                    )
                }
                AnyKeyword::Const => node.constant = Some(value.clone()),
            },
            SchemaKeyword::Object(keyword) => match keyword {
                ObjectKeyword::Properties => {
                    let properties = value
                        .as_map()
                        .ok_or_else(|| {
                            structure_error("`properties` must be an object".to_string())
                        })?
                        .iter()
                        .map(|(property_name, property)| {
                            Ok((
                                text(property_name, name)?.to_string(),
                                build_node(property, false, pending)?,
                            ))
                        })
                        .collect::<Result<_, DataContractError>>()?;
                    node.properties = Some(properties);
                }
                ObjectKeyword::AdditionalProperties => {
                    node.additional_properties = Some(boolean(value, name)?)
                }
                ObjectKeyword::Required => node.required = Some(texts_of(value, name)?),
                ObjectKeyword::MinProperties => node.min_properties = Some(count(value, name)?),
                ObjectKeyword::MaxProperties => node.max_properties = Some(count(value, name)?),
                ObjectKeyword::DependentRequired => {
                    let dependent_required = value
                        .as_map()
                        .ok_or_else(|| {
                            structure_error("`dependentRequired` must be an object".to_string())
                        })?
                        .iter()
                        .map(|(property_name, names)| {
                            Ok((
                                text(property_name, name)?.to_string(),
                                texts_of(names, name)?,
                            ))
                        })
                        .collect::<Result<_, DataContractError>>()?;
                    node.dependent_required = Some(dependent_required);
                }
            },
            SchemaKeyword::Array(keyword) => match keyword {
                ArrayKeyword::Items => {
                    node.items = Some(Box::new(build_node(value, false, pending)?))
                }
                ArrayKeyword::Contains => {
                    node.contains = Some(Box::new(build_node(value, false, pending)?))
                }
                ArrayKeyword::MinItems => node.min_items = Some(count(value, name)?),
                ArrayKeyword::MaxItems => node.max_items = Some(count(value, name)?),
                ArrayKeyword::UniqueItems => node.unique_items = Some(boolean(value, name)?),
            },
            SchemaKeyword::String(keyword) => match keyword {
                StringKeyword::MinLength => node.min_length = Some(count(value, name)?),
                StringKeyword::MaxLength => node.max_length = Some(count(value, name)?),
                StringKeyword::MaxBytes => node.max_bytes = Some(count(value, name)?),
                StringKeyword::Pattern => node.pattern = Some(text(value, name)?.to_string()),
            },
            SchemaKeyword::Number(keyword) => {
                let bound = Some(number(value, name)?);
                match keyword {
                    NumberKeyword::Minimum => node.minimum = bound,
                    NumberKeyword::Maximum => node.maximum = bound,
                    NumberKeyword::ExclusiveMinimum => node.exclusive_minimum = bound,
                    NumberKeyword::ExclusiveMaximum => node.exclusive_maximum = bound,
                    NumberKeyword::MultipleOf => node.multiple_of = bound,
                }
            }
            SchemaKeyword::Format(keyword) => match keyword {
                FormatKeyword::Format => node.format = Some(text(value, name)?.to_string()),
                FormatKeyword::ByteArray => node.byte_array = Some(boolean(value, name)?),
                FormatKeyword::ContentMediaType => {
                    node.content_media_type = Some(text(value, name)?.to_string())
                }
                FormatKeyword::EncryptedFor => {
                    node.encrypted_for =
                        Some(platform_value::from_value(value.clone()).map_err(|error| {
                            structure_error(format!("`{name}` is malformed: {error}"))
                        })?)
                }
            },
            SchemaKeyword::Relationship(_)
            | SchemaKeyword::Meta(_)
            | SchemaKeyword::Annotation(_) => {}
        }
    }
    Ok(node)
}

/// Resolves a `$ref` against the schema it appears in: a fragment-only URI holding a JSON
/// pointer, its segments percent-decoded and then unescaped (`~1` is `/`, `~0` is `~`).
fn resolve_pointer<'a>(root: &'a Value, reference: &str) -> Result<&'a Value, DataContractError> {
    let unresolved = || structure_error(format!("`$ref` `{reference}` does not resolve"));
    let pointer = reference.strip_prefix('#').ok_or_else(unresolved)?;
    if pointer.is_empty() {
        return Ok(root);
    }
    let pointer = pointer.strip_prefix('/').ok_or_else(unresolved)?;
    let mut target = root;
    for segment in pointer.split('/') {
        let segment = percent_decode(segment)
            .ok_or_else(unresolved)?
            .replace("~1", "/")
            .replace("~0", "~");
        target = match target {
            Value::Map(map) => map
                .iter()
                .find(|(key, _)| key.as_text() == Some(segment.as_str()))
                .map(|(_, value)| value)
                .ok_or_else(unresolved)?,
            Value::Array(elements) => segment
                .parse::<usize>()
                .ok()
                .and_then(|index| elements.get(index))
                .ok_or_else(unresolved)?,
            _ => return Err(unresolved()),
        };
    }
    Ok(target)
}

fn percent_decode(segment: &str) -> Option<String> {
    let bytes = segment.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = segment.get(index + 1..index + 3)?;
            if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return None;
            }
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}
