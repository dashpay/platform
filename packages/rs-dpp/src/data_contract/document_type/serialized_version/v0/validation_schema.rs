//! The stored form of a document type's typed validation schema (`ValidationSchema`), with the
//! generation of the builder that built it. The schemas `$ref`s reach are kept once each, in
//! [`StoredValidationSchema::nodes`], and every reference names its node by index, as the
//! built schema shares one node among the references that reach it.
//!
//! The stored form decodes without the `validation-schema` feature; converting it to and from
//! the built schema needs the feature.

use super::nested::Nested;
use super::property::StoredEncryptedFor;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Value;
use std::collections::BTreeMap;

#[cfg(feature = "validation-schema")]
use super::stored_error;
#[cfg(feature = "validation-schema")]
use crate::data_contract::document_type::schema::validation_schema::{
    ArrayConstraints, FormatConstraints, JsonType, JsonTypes, NumberConstraints, ObjectConstraints,
    SchemaNode, StringConstraints, ValidationSchema, ValidatorFormat,
};
#[cfg(feature = "validation-schema")]
use crate::data_contract::document_type::EncryptedFor;
#[cfg(feature = "validation-schema")]
use crate::ProtocolError;
#[cfg(feature = "validation-schema")]
use platform_version::version::FeatureVersion;
#[cfg(feature = "validation-schema")]
use std::sync::Arc;

/// `ValidationSchema`, and the builder generation that built it.
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredValidationSchema {
    generation: u16,
    root: StoredSchemaNode,
    nodes: Vec<StoredSchemaNode>,
    references: BTreeMap<String, u32>,
}

/// `SchemaNode`.
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredSchemaNode {
    reference: Option<String>,
    types: Option<StoredJsonTypes>,
    enum_values: Option<Vec<Value>>,
    constant: Option<Value>,
    object: Option<StoredObjectConstraints>,
    array: Option<StoredArrayConstraints>,
    string: Option<StoredStringConstraints>,
    number: Option<StoredNumberConstraints>,
    format: Option<StoredFormatConstraints>,
}

/// `JsonTypes`.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredJsonTypes {
    One(StoredJsonType),
    AnyOf(Vec<StoredJsonType>),
}

/// `JsonType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredJsonType {
    Null,
    Boolean,
    Object,
    Array,
    Number,
    String,
    Integer,
}

/// `ObjectConstraints`.
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredObjectConstraints {
    properties: Option<Nested<Vec<(String, StoredSchemaNode)>>>,
    additional_properties: Option<bool>,
    required: Option<Vec<String>>,
    min_properties: Option<u64>,
    max_properties: Option<u64>,
    dependent_required: Option<BTreeMap<String, Vec<String>>>,
}

/// `ArrayConstraints`.
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredArrayConstraints {
    items: Option<Box<Nested<StoredSchemaNode>>>,
    contains: Option<Box<Nested<StoredSchemaNode>>>,
    min_items: Option<u64>,
    max_items: Option<u64>,
    unique_items: Option<bool>,
    byte_array: bool,
}

/// `StringConstraints`.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredStringConstraints {
    min_length: Option<u64>,
    max_length: Option<u64>,
    max_bytes: Option<u64>,
    pattern: Option<String>,
}

/// `NumberConstraints`.
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredNumberConstraints {
    minimum: Option<Value>,
    maximum: Option<Value>,
    exclusive_minimum: Option<Value>,
    exclusive_maximum: Option<Value>,
    multiple_of: Option<Value>,
}

/// `FormatConstraints`.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredFormatConstraints {
    format: Option<StoredValidatorFormat>,
    encrypted_for: Option<StoredEncryptedFor>,
}

/// `ValidatorFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredValidatorFormat {
    Date,
    DateTime,
    Email,
    Hostname,
    IdnEmail,
    Ipv4,
    Ipv6,
    Regex,
    Time,
    Uri,
}

#[cfg(feature = "validation-schema")]
impl StoredValidationSchema {
    /// `schema`, which builder generation `generation` built.
    pub(super) fn new(schema: &ValidationSchema, generation: FeatureVersion) -> Self {
        let mut nodes = Vec::new();
        let mut indexes: BTreeMap<*const SchemaNode, u32> = BTreeMap::new();
        let references = schema
            .references
            .iter()
            .map(|(reference, node)| {
                let index = *indexes.entry(Arc::as_ptr(node)).or_insert_with(|| {
                    nodes.push(StoredSchemaNode::from(node.as_ref()));
                    // A schema holds far fewer than 2^32 nodes
                    (nodes.len() - 1) as u32
                });
                (reference.clone(), index)
            })
            .collect();
        StoredValidationSchema {
            generation,
            root: StoredSchemaNode::from(&schema.root),
            nodes,
            references,
        }
    }

    /// The built schema and the generation that built it. Refuses a reference to a node the
    /// stored form does not hold.
    pub(super) fn into_validation_schema(
        self,
    ) -> Result<(ValidationSchema, FeatureVersion), ProtocolError> {
        let StoredValidationSchema {
            generation,
            root,
            nodes,
            references,
        } = self;
        let nodes: Vec<Arc<SchemaNode>> = nodes
            .into_iter()
            .map(|node| Arc::new(node.into()))
            .collect();
        let references = references
            .into_iter()
            .map(|(reference, index)| {
                let node = nodes.get(index as usize).ok_or_else(|| {
                    stored_error(format!(
                        "reference {reference} names node {index}, which is not stored"
                    ))
                })?;
                Ok((reference, Arc::clone(node)))
            })
            .collect::<Result<_, ProtocolError>>()?;
        Ok((
            ValidationSchema {
                root: root.into(),
                references,
            },
            generation,
        ))
    }
}

#[cfg(feature = "validation-schema")]
impl From<&SchemaNode> for StoredSchemaNode {
    fn from(node: &SchemaNode) -> Self {
        let SchemaNode {
            reference,
            types,
            enum_values,
            constant,
            object,
            array,
            string,
            number,
            format,
        } = node;
        StoredSchemaNode {
            reference: reference.clone(),
            types: types.as_ref().map(|types| match types {
                JsonTypes::One(json_type) => StoredJsonTypes::One((*json_type).into()),
                JsonTypes::AnyOf(json_types) => StoredJsonTypes::AnyOf(
                    json_types
                        .iter()
                        .map(|json_type| (*json_type).into())
                        .collect(),
                ),
            }),
            enum_values: enum_values.clone(),
            constant: constant.clone(),
            object: object.as_deref().map(|object| StoredObjectConstraints {
                properties: object.properties.as_ref().map(|properties| {
                    Nested(
                        properties
                            .iter()
                            .map(|(name, node)| (name.clone(), StoredSchemaNode::from(node)))
                            .collect(),
                    )
                }),
                additional_properties: object.additional_properties,
                required: object.required.clone(),
                min_properties: object.min_properties,
                max_properties: object.max_properties,
                dependent_required: object.dependent_required.clone(),
            }),
            array: array.as_deref().map(|array| StoredArrayConstraints {
                items: array
                    .items
                    .as_deref()
                    .map(|node| Box::new(Nested(StoredSchemaNode::from(node)))),
                contains: array
                    .contains
                    .as_deref()
                    .map(|node| Box::new(Nested(StoredSchemaNode::from(node)))),
                min_items: array.min_items,
                max_items: array.max_items,
                unique_items: array.unique_items,
                byte_array: array.byte_array,
            }),
            string: string.as_deref().map(|string| StoredStringConstraints {
                min_length: string.min_length,
                max_length: string.max_length,
                max_bytes: string.max_bytes,
                pattern: string.pattern.clone(),
            }),
            number: number.as_deref().map(|number| StoredNumberConstraints {
                minimum: number.minimum.clone(),
                maximum: number.maximum.clone(),
                exclusive_minimum: number.exclusive_minimum.clone(),
                exclusive_maximum: number.exclusive_maximum.clone(),
                multiple_of: number.multiple_of.clone(),
            }),
            format: format.as_deref().map(|format| StoredFormatConstraints {
                format: format.format.map(Into::into),
                encrypted_for: format.encrypted_for.as_ref().map(StoredEncryptedFor::from),
            }),
        }
    }
}

#[cfg(feature = "validation-schema")]
impl From<StoredSchemaNode> for SchemaNode {
    fn from(stored: StoredSchemaNode) -> Self {
        let StoredSchemaNode {
            reference,
            types,
            enum_values,
            constant,
            object,
            array,
            string,
            number,
            format,
        } = stored;
        SchemaNode {
            reference,
            types: types.map(|types| match types {
                StoredJsonTypes::One(json_type) => JsonTypes::One(json_type.into()),
                StoredJsonTypes::AnyOf(json_types) => {
                    JsonTypes::AnyOf(json_types.into_iter().map(Into::into).collect())
                }
            }),
            enum_values,
            constant,
            object: object.map(|object| {
                Box::new(ObjectConstraints {
                    properties: object.properties.map(|properties| {
                        properties
                            .0
                            .into_iter()
                            .map(|(name, node)| (name, SchemaNode::from(node)))
                            .collect()
                    }),
                    additional_properties: object.additional_properties,
                    required: object.required,
                    min_properties: object.min_properties,
                    max_properties: object.max_properties,
                    dependent_required: object.dependent_required,
                })
            }),
            array: array.map(|array| {
                Box::new(ArrayConstraints {
                    items: array.items.map(|node| Box::new(SchemaNode::from(node.0))),
                    contains: array
                        .contains
                        .map(|node| Box::new(SchemaNode::from(node.0))),
                    min_items: array.min_items,
                    max_items: array.max_items,
                    unique_items: array.unique_items,
                    byte_array: array.byte_array,
                })
            }),
            string: string.map(|string| {
                Box::new(StringConstraints {
                    min_length: string.min_length,
                    max_length: string.max_length,
                    max_bytes: string.max_bytes,
                    pattern: string.pattern,
                })
            }),
            number: number.map(|number| {
                Box::new(NumberConstraints {
                    minimum: number.minimum,
                    maximum: number.maximum,
                    exclusive_minimum: number.exclusive_minimum,
                    exclusive_maximum: number.exclusive_maximum,
                    multiple_of: number.multiple_of,
                })
            }),
            format: format.map(|format| {
                Box::new(FormatConstraints {
                    format: format.format.map(Into::into),
                    encrypted_for: format.encrypted_for.map(EncryptedFor::from),
                })
            }),
        }
    }
}

#[cfg(feature = "validation-schema")]
impl From<JsonType> for StoredJsonType {
    fn from(json_type: JsonType) -> Self {
        match json_type {
            JsonType::Null => StoredJsonType::Null,
            JsonType::Boolean => StoredJsonType::Boolean,
            JsonType::Object => StoredJsonType::Object,
            JsonType::Array => StoredJsonType::Array,
            JsonType::Number => StoredJsonType::Number,
            JsonType::String => StoredJsonType::String,
            JsonType::Integer => StoredJsonType::Integer,
        }
    }
}

#[cfg(feature = "validation-schema")]
impl From<StoredJsonType> for JsonType {
    fn from(stored: StoredJsonType) -> Self {
        match stored {
            StoredJsonType::Null => JsonType::Null,
            StoredJsonType::Boolean => JsonType::Boolean,
            StoredJsonType::Object => JsonType::Object,
            StoredJsonType::Array => JsonType::Array,
            StoredJsonType::Number => JsonType::Number,
            StoredJsonType::String => JsonType::String,
            StoredJsonType::Integer => JsonType::Integer,
        }
    }
}

#[cfg(feature = "validation-schema")]
impl From<ValidatorFormat> for StoredValidatorFormat {
    fn from(format: ValidatorFormat) -> Self {
        match format {
            ValidatorFormat::Date => StoredValidatorFormat::Date,
            ValidatorFormat::DateTime => StoredValidatorFormat::DateTime,
            ValidatorFormat::Email => StoredValidatorFormat::Email,
            ValidatorFormat::Hostname => StoredValidatorFormat::Hostname,
            ValidatorFormat::IdnEmail => StoredValidatorFormat::IdnEmail,
            ValidatorFormat::Ipv4 => StoredValidatorFormat::Ipv4,
            ValidatorFormat::Ipv6 => StoredValidatorFormat::Ipv6,
            ValidatorFormat::Regex => StoredValidatorFormat::Regex,
            ValidatorFormat::Time => StoredValidatorFormat::Time,
            ValidatorFormat::Uri => StoredValidatorFormat::Uri,
        }
    }
}

#[cfg(feature = "validation-schema")]
impl From<StoredValidatorFormat> for ValidatorFormat {
    fn from(stored: StoredValidatorFormat) -> Self {
        match stored {
            StoredValidatorFormat::Date => ValidatorFormat::Date,
            StoredValidatorFormat::DateTime => ValidatorFormat::DateTime,
            StoredValidatorFormat::Email => ValidatorFormat::Email,
            StoredValidatorFormat::Hostname => ValidatorFormat::Hostname,
            StoredValidatorFormat::IdnEmail => ValidatorFormat::IdnEmail,
            StoredValidatorFormat::Ipv4 => ValidatorFormat::Ipv4,
            StoredValidatorFormat::Ipv6 => ValidatorFormat::Ipv6,
            StoredValidatorFormat::Regex => ValidatorFormat::Regex,
            StoredValidatorFormat::Time => ValidatorFormat::Time,
            StoredValidatorFormat::Uri => ValidatorFormat::Uri,
        }
    }
}
