//! The stored form of a document type's properties: [`DocumentProperty`] and every type its
//! property types hold. Property maps are kept as ordered lists, in the order the parse gave
//! them, since that order is read (a document's encoding, `flattened_properties`' walk).

use super::nested::Nested;
use super::stored_error;
use crate::data_contract::document_type::array::{
    ArrayItemConstraints, ArrayItemType, TypedArrayProperty,
};
use crate::data_contract::document_type::{
    ByteArrayPropertySizes, DistinctFrom, DocumentProperty, DocumentPropertyReferenceTarget,
    DocumentPropertyType, EncryptedFor, EncryptedForRecipient, EncryptionScheme, GeneratedFrom,
    GenerationParam, HashFunction, IdentityKeyReferenceRequirements, KeyIdReference,
    KeyReferenceIdentityProperty, StringPropertySizes, StringTransformation, SystemFunction,
    TokenFunction,
};
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use indexmap::IndexMap;
use platform_value::Value;

/// A property map, in order.
pub(super) type StoredProperties = Vec<(String, StoredProperty)>;

/// [`DocumentProperty`].
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredProperty {
    property_type: StoredPropertyType,
    required: bool,
    transient: bool,
    required_since: Option<u32>,
    distinct_from: Option<StoredDistinctFrom>,
    encrypted_for: Option<StoredEncryptedFor>,
    generated_from: Option<StoredGeneratedFrom>,
    revealed_reference: Option<DocumentPropertyReferenceTarget>,
}

/// [`DocumentPropertyType`].
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredPropertyType {
    U128,
    I128,
    U64,
    I64,
    U32,
    I32,
    U16,
    I16,
    U8,
    I8,
    F64,
    String {
        min_length: Option<u16>,
        max_length: Option<u16>,
        max_bytes: Option<u16>,
    },
    ByteArray {
        min_size: Option<u16>,
        max_size: Option<u16>,
    },
    Identifier,
    Boolean,
    Date,
    Object(Nested<StoredProperties>),
    Array(StoredArrayItemType),
    VariableTypeArray(Vec<StoredArrayItemType>),
    IdentifierWithReference(DocumentPropertyReferenceTarget),
    TypedArray(Box<Nested<StoredTypedArray>>),
    KeyIdWithReference {
        identity_property: StoredKeyIdentityProperty,
        key_requirements: IdentityKeyReferenceRequirements,
    },
}

/// [`ArrayItemType`], its sizes as `u64` whatever the platform's `usize`.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredArrayItemType {
    Integer,
    Number,
    String(Option<u64>, Option<u64>),
    ByteArray(Option<u64>, Option<u64>),
    Identifier,
    Boolean,
    Date,
}

/// [`TypedArrayProperty`].
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredTypedArray {
    item_type: StoredPropertyType,
    allowed_values: Option<Vec<Value>>,
    minimum: Option<Value>,
    maximum: Option<Value>,
    min_items: Option<u16>,
    max_items: u16,
    unique_items: bool,
}

/// [`KeyReferenceIdentityProperty`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredKeyIdentityProperty {
    OwnerId,
    CreatorId,
    Property(String),
}

/// [`DistinctFrom`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredDistinctFrom {
    OwnerId,
    Property(String),
}

/// [`EncryptedFor`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredEncryptedFor {
    recipient: StoredEncryptedForRecipient,
    recipient_key: String,
    sender_key: String,
    scheme: StoredEncryptionScheme,
}

/// [`EncryptedForRecipient`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredEncryptedForRecipient {
    Owner,
    Property(String),
}

/// [`EncryptionScheme`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredEncryptionScheme {
    EcdhSecp256k1Aes256Cbc,
}

/// [`GeneratedFrom`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredGeneratedFrom {
    function: StoredSystemFunction,
    params: Vec<String>,
}

/// [`SystemFunction`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredSystemFunction {
    StringTransformation(StoredStringTransformation),
    Hash(HashFunction),
    Token(StoredTokenFunction),
}

/// [`TokenFunction`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredTokenFunction {
    Decimals,
}

/// [`StringTransformation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredStringTransformation {
    CamelCase,
    Capitalize,
    HomographSafeAscii,
    Lowercase,
    SnakeCase,
    Uppercase,
}

/// `properties` in order.
pub(super) fn stored_properties(
    properties: &IndexMap<String, DocumentProperty>,
) -> StoredProperties {
    properties
        .iter()
        .map(|(name, property)| (name.clone(), StoredProperty::from(property)))
        .collect()
}

/// The property map `stored` holds, in order.
pub(super) fn properties_from_stored(
    stored: StoredProperties,
) -> Result<IndexMap<String, DocumentProperty>, ProtocolError> {
    let mut properties = IndexMap::with_capacity(stored.len());
    for (name, property) in stored {
        if properties
            .insert(name.clone(), DocumentProperty::try_from(property)?)
            .is_some()
        {
            return Err(stored_error(format!("property {name} is stored twice")));
        }
    }
    Ok(properties)
}

impl From<&DocumentProperty> for StoredProperty {
    fn from(property: &DocumentProperty) -> Self {
        let DocumentProperty {
            property_type,
            required,
            transient,
            required_since,
            distinct_from,
            encrypted_for,
            generated_from,
            revealed_reference,
        } = property;
        StoredProperty {
            property_type: property_type.into(),
            required: *required,
            transient: *transient,
            required_since: *required_since,
            distinct_from: distinct_from
                .as_ref()
                .map(|distinct_from| match distinct_from {
                    DistinctFrom::OwnerId => StoredDistinctFrom::OwnerId,
                    DistinctFrom::Property(path) => StoredDistinctFrom::Property(path.clone()),
                }),
            encrypted_for: encrypted_for.as_ref().map(StoredEncryptedFor::from),
            generated_from: generated_from.as_ref().map(StoredGeneratedFrom::from),
            revealed_reference: revealed_reference.clone(),
        }
    }
}

impl TryFrom<StoredProperty> for DocumentProperty {
    type Error = ProtocolError;

    fn try_from(stored: StoredProperty) -> Result<Self, Self::Error> {
        let StoredProperty {
            property_type,
            required,
            transient,
            required_since,
            distinct_from,
            encrypted_for,
            generated_from,
            revealed_reference,
        } = stored;
        Ok(DocumentProperty {
            property_type: property_type.try_into()?,
            required,
            transient,
            required_since,
            distinct_from: distinct_from.map(|distinct_from| match distinct_from {
                StoredDistinctFrom::OwnerId => DistinctFrom::OwnerId,
                StoredDistinctFrom::Property(path) => DistinctFrom::Property(path),
            }),
            encrypted_for: encrypted_for.map(EncryptedFor::from),
            generated_from: generated_from.map(GeneratedFrom::from),
            revealed_reference,
        })
    }
}

impl From<&DocumentPropertyType> for StoredPropertyType {
    fn from(property_type: &DocumentPropertyType) -> Self {
        match property_type {
            DocumentPropertyType::U128 => StoredPropertyType::U128,
            DocumentPropertyType::I128 => StoredPropertyType::I128,
            DocumentPropertyType::U64 => StoredPropertyType::U64,
            DocumentPropertyType::I64 => StoredPropertyType::I64,
            DocumentPropertyType::U32 => StoredPropertyType::U32,
            DocumentPropertyType::I32 => StoredPropertyType::I32,
            DocumentPropertyType::U16 => StoredPropertyType::U16,
            DocumentPropertyType::I16 => StoredPropertyType::I16,
            DocumentPropertyType::U8 => StoredPropertyType::U8,
            DocumentPropertyType::I8 => StoredPropertyType::I8,
            DocumentPropertyType::F64 => StoredPropertyType::F64,
            DocumentPropertyType::String(StringPropertySizes {
                min_length,
                max_length,
                max_bytes,
            }) => StoredPropertyType::String {
                min_length: *min_length,
                max_length: *max_length,
                max_bytes: *max_bytes,
            },
            DocumentPropertyType::ByteArray(ByteArrayPropertySizes { min_size, max_size }) => {
                StoredPropertyType::ByteArray {
                    min_size: *min_size,
                    max_size: *max_size,
                }
            }
            DocumentPropertyType::Identifier => StoredPropertyType::Identifier,
            DocumentPropertyType::Boolean => StoredPropertyType::Boolean,
            DocumentPropertyType::Date => StoredPropertyType::Date,
            DocumentPropertyType::Object(properties) => {
                StoredPropertyType::Object(Nested(stored_properties(properties)))
            }
            DocumentPropertyType::Array(item_type) => StoredPropertyType::Array(item_type.into()),
            DocumentPropertyType::VariableTypeArray(item_types) => {
                StoredPropertyType::VariableTypeArray(
                    item_types.iter().map(StoredArrayItemType::from).collect(),
                )
            }
            DocumentPropertyType::IdentifierWithReference(target) => {
                StoredPropertyType::IdentifierWithReference(target.clone())
            }
            DocumentPropertyType::TypedArray(TypedArrayProperty {
                item_type,
                item_constraints:
                    ArrayItemConstraints {
                        allowed_values,
                        minimum,
                        maximum,
                    },
                min_items,
                max_items,
                unique_items,
            }) => StoredPropertyType::TypedArray(Box::new(Nested(StoredTypedArray {
                item_type: item_type.as_ref().into(),
                allowed_values: allowed_values.clone(),
                minimum: minimum.clone(),
                maximum: maximum.clone(),
                min_items: *min_items,
                max_items: *max_items,
                unique_items: *unique_items,
            }))),
            DocumentPropertyType::KeyIdWithReference(KeyIdReference {
                identity_property,
                key_requirements,
            }) => StoredPropertyType::KeyIdWithReference {
                identity_property: match identity_property {
                    KeyReferenceIdentityProperty::OwnerId => StoredKeyIdentityProperty::OwnerId,
                    KeyReferenceIdentityProperty::CreatorId => StoredKeyIdentityProperty::CreatorId,
                    KeyReferenceIdentityProperty::Property(path) => {
                        StoredKeyIdentityProperty::Property(path.clone())
                    }
                },
                key_requirements: key_requirements.clone(),
            },
        }
    }
}

impl TryFrom<StoredPropertyType> for DocumentPropertyType {
    type Error = ProtocolError;

    fn try_from(stored: StoredPropertyType) -> Result<Self, Self::Error> {
        Ok(match stored {
            StoredPropertyType::U128 => DocumentPropertyType::U128,
            StoredPropertyType::I128 => DocumentPropertyType::I128,
            StoredPropertyType::U64 => DocumentPropertyType::U64,
            StoredPropertyType::I64 => DocumentPropertyType::I64,
            StoredPropertyType::U32 => DocumentPropertyType::U32,
            StoredPropertyType::I32 => DocumentPropertyType::I32,
            StoredPropertyType::U16 => DocumentPropertyType::U16,
            StoredPropertyType::I16 => DocumentPropertyType::I16,
            StoredPropertyType::U8 => DocumentPropertyType::U8,
            StoredPropertyType::I8 => DocumentPropertyType::I8,
            StoredPropertyType::F64 => DocumentPropertyType::F64,
            StoredPropertyType::String {
                min_length,
                max_length,
                max_bytes,
            } => DocumentPropertyType::String(StringPropertySizes {
                min_length,
                max_length,
                max_bytes,
            }),
            StoredPropertyType::ByteArray { min_size, max_size } => {
                DocumentPropertyType::ByteArray(ByteArrayPropertySizes { min_size, max_size })
            }
            StoredPropertyType::Identifier => DocumentPropertyType::Identifier,
            StoredPropertyType::Boolean => DocumentPropertyType::Boolean,
            StoredPropertyType::Date => DocumentPropertyType::Date,
            StoredPropertyType::Object(Nested(properties)) => {
                DocumentPropertyType::Object(properties_from_stored(properties)?)
            }
            StoredPropertyType::Array(item_type) => {
                DocumentPropertyType::Array(item_type.try_into()?)
            }
            StoredPropertyType::VariableTypeArray(item_types) => {
                DocumentPropertyType::VariableTypeArray(
                    item_types
                        .into_iter()
                        .map(ArrayItemType::try_from)
                        .collect::<Result<_, _>>()?,
                )
            }
            StoredPropertyType::IdentifierWithReference(target) => {
                DocumentPropertyType::IdentifierWithReference(target)
            }
            StoredPropertyType::TypedArray(typed_array) => {
                let Nested(StoredTypedArray {
                    item_type,
                    allowed_values,
                    minimum,
                    maximum,
                    min_items,
                    max_items,
                    unique_items,
                }) = *typed_array;
                DocumentPropertyType::TypedArray(TypedArrayProperty {
                    item_type: Box::new(item_type.try_into()?),
                    item_constraints: ArrayItemConstraints {
                        allowed_values,
                        minimum,
                        maximum,
                    },
                    min_items,
                    max_items,
                    unique_items,
                })
            }
            StoredPropertyType::KeyIdWithReference {
                identity_property,
                key_requirements,
            } => DocumentPropertyType::KeyIdWithReference(KeyIdReference {
                identity_property: match identity_property {
                    StoredKeyIdentityProperty::OwnerId => KeyReferenceIdentityProperty::OwnerId,
                    StoredKeyIdentityProperty::CreatorId => KeyReferenceIdentityProperty::CreatorId,
                    StoredKeyIdentityProperty::Property(path) => {
                        KeyReferenceIdentityProperty::Property(path)
                    }
                },
                key_requirements,
            }),
        })
    }
}

impl From<&ArrayItemType> for StoredArrayItemType {
    fn from(item_type: &ArrayItemType) -> Self {
        // `usize` is at most 64 bits on every platform Platform builds for
        let size = |size: &Option<usize>| size.map(|size| size as u64);
        match item_type {
            ArrayItemType::Integer => StoredArrayItemType::Integer,
            ArrayItemType::Number => StoredArrayItemType::Number,
            ArrayItemType::String(min, max) => StoredArrayItemType::String(size(min), size(max)),
            ArrayItemType::ByteArray(min, max) => {
                StoredArrayItemType::ByteArray(size(min), size(max))
            }
            ArrayItemType::Identifier => StoredArrayItemType::Identifier,
            ArrayItemType::Boolean => StoredArrayItemType::Boolean,
            ArrayItemType::Date => StoredArrayItemType::Date,
        }
    }
}

impl TryFrom<StoredArrayItemType> for ArrayItemType {
    type Error = ProtocolError;

    fn try_from(stored: StoredArrayItemType) -> Result<Self, Self::Error> {
        let size = |size: Option<u64>| {
            size.map(|size| {
                usize::try_from(size).map_err(|_| {
                    stored_error(format!(
                        "array element size {size} does not fit this platform"
                    ))
                })
            })
            .transpose()
        };
        Ok(match stored {
            StoredArrayItemType::Integer => ArrayItemType::Integer,
            StoredArrayItemType::Number => ArrayItemType::Number,
            StoredArrayItemType::String(min, max) => ArrayItemType::String(size(min)?, size(max)?),
            StoredArrayItemType::ByteArray(min, max) => {
                ArrayItemType::ByteArray(size(min)?, size(max)?)
            }
            StoredArrayItemType::Identifier => ArrayItemType::Identifier,
            StoredArrayItemType::Boolean => ArrayItemType::Boolean,
            StoredArrayItemType::Date => ArrayItemType::Date,
        })
    }
}

impl From<&EncryptedFor> for StoredEncryptedFor {
    fn from(encrypted_for: &EncryptedFor) -> Self {
        let EncryptedFor {
            recipient,
            recipient_key,
            sender_key,
            scheme,
        } = encrypted_for;
        StoredEncryptedFor {
            recipient: match recipient {
                EncryptedForRecipient::Owner => StoredEncryptedForRecipient::Owner,
                EncryptedForRecipient::Property(path) => {
                    StoredEncryptedForRecipient::Property(path.clone())
                }
            },
            recipient_key: recipient_key.clone(),
            sender_key: sender_key.clone(),
            scheme: match scheme {
                EncryptionScheme::EcdhSecp256k1Aes256Cbc => {
                    StoredEncryptionScheme::EcdhSecp256k1Aes256Cbc
                }
            },
        }
    }
}

impl From<StoredEncryptedFor> for EncryptedFor {
    fn from(stored: StoredEncryptedFor) -> Self {
        let StoredEncryptedFor {
            recipient,
            recipient_key,
            sender_key,
            scheme,
        } = stored;
        EncryptedFor {
            recipient: match recipient {
                StoredEncryptedForRecipient::Owner => EncryptedForRecipient::Owner,
                StoredEncryptedForRecipient::Property(path) => {
                    EncryptedForRecipient::Property(path)
                }
            },
            recipient_key,
            sender_key,
            scheme: match scheme {
                StoredEncryptionScheme::EcdhSecp256k1Aes256Cbc => {
                    EncryptionScheme::EcdhSecp256k1Aes256Cbc
                }
            },
        }
    }
}

impl From<&GeneratedFrom> for StoredGeneratedFrom {
    fn from(generated_from: &GeneratedFrom) -> Self {
        let GeneratedFrom { function, params } = generated_from;
        StoredGeneratedFrom {
            function: match function {
                SystemFunction::StringTransformation(transformation) => {
                    StoredSystemFunction::StringTransformation(match transformation {
                        StringTransformation::CamelCase => StoredStringTransformation::CamelCase,
                        StringTransformation::Capitalize => StoredStringTransformation::Capitalize,
                        StringTransformation::HomographSafeAscii => {
                            StoredStringTransformation::HomographSafeAscii
                        }
                        StringTransformation::Lowercase => StoredStringTransformation::Lowercase,
                        StringTransformation::SnakeCase => StoredStringTransformation::SnakeCase,
                        StringTransformation::Uppercase => StoredStringTransformation::Uppercase,
                    })
                }
                SystemFunction::Hash(hash) => StoredSystemFunction::Hash(*hash),
                SystemFunction::Token(token) => StoredSystemFunction::Token(match token {
                    TokenFunction::Decimals => StoredTokenFunction::Decimals,
                }),
            },
            params: params
                .iter()
                .map(|param| match param {
                    GenerationParam::Property(path) => path.clone(),
                })
                .collect(),
        }
    }
}

impl From<StoredGeneratedFrom> for GeneratedFrom {
    fn from(stored: StoredGeneratedFrom) -> Self {
        let StoredGeneratedFrom { function, params } = stored;
        GeneratedFrom {
            function: match function {
                StoredSystemFunction::StringTransformation(transformation) => {
                    SystemFunction::StringTransformation(match transformation {
                        StoredStringTransformation::CamelCase => StringTransformation::CamelCase,
                        StoredStringTransformation::Capitalize => StringTransformation::Capitalize,
                        StoredStringTransformation::HomographSafeAscii => {
                            StringTransformation::HomographSafeAscii
                        }
                        StoredStringTransformation::Lowercase => StringTransformation::Lowercase,
                        StoredStringTransformation::SnakeCase => StringTransformation::SnakeCase,
                        StoredStringTransformation::Uppercase => StringTransformation::Uppercase,
                    })
                }
                StoredSystemFunction::Hash(hash) => SystemFunction::Hash(hash),
                StoredSystemFunction::Token(token) => SystemFunction::Token(match token {
                    StoredTokenFunction::Decimals => TokenFunction::Decimals,
                }),
            },
            params: params.into_iter().map(GenerationParam::Property).collect(),
        }
    }
}
