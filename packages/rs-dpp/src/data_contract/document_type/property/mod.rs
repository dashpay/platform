use std::collections::BTreeMap;
use std::convert::TryInto;

use std::io::{BufReader, Cursor, Read};

use crate::data_contract::errors::DataContractError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use crate::consensus::basic::decode::DecodingError;
use crate::data_contract::config::v1::DataContractConfigGettersV1;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::property_names;
use crate::prelude::TimestampMillis;
use crate::ProtocolError;
use array::ArrayItemType;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use integer_encoding::{VarInt, VarIntReader};
use itertools::Itertools;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use rand::distributions::{Alphanumeric, Standard};
use rand::rngs::StdRng;
use rand::Rng;
use serde::Serialize;

pub mod array;

#[cfg(test)]
mod byte_array_encoding_flip_tests;

// This struct will be changed in future to support more validation logic and serialization
// It will become versioned and it will be introduced by a new document type version
// @append_only
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct DocumentProperty {
    pub property_type: DocumentPropertyType,
    pub required: bool,
    pub transient: bool,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct StringPropertySizes {
    pub min_length: Option<u16>,
    pub max_length: Option<u16>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct ByteArrayPropertySizes {
    pub min_size: Option<u16>,
    pub max_size: Option<u16>,
}

// This enum is embedded in consensus errors, so it is consensus-serialized.
// @append_only
#[derive(
    Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum DocumentPropertyReferenceTarget {
    Identity,
    Contract,
    Token,
    /// A document of a document type whose documents can never be deleted
    /// (`canBeDeleted: false`). Only such document types may be referenced:
    /// together with document types being non-removable and the
    /// `canBeDeleted` flag being immutable on contract updates, this
    /// guarantees a validated reference can never dangle.
    #[serde(rename = "permanentDocument")]
    PermanentDocument {
        /// The contract the referenced document type lives in; `None` means
        /// the declaring contract itself
        contract_id: Option<Identifier>,
        document_type_name: String,
    },
}

impl std::fmt::Display for DocumentPropertyReferenceTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentPropertyReferenceTarget::Identity => write!(f, "identity"),
            DocumentPropertyReferenceTarget::Contract => write!(f, "contract"),
            DocumentPropertyReferenceTarget::Token => write!(f, "token"),
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: Some(contract_id),
                document_type_name,
            } => write!(
                f,
                "permanent document (contract {contract_id}, document type {document_type_name})"
            ),
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: None,
                document_type_name,
            } => write!(
                f,
                "permanent document (own contract, document type {document_type_name})"
            ),
        }
    }
}

// @append_only
#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum DocumentPropertyType {
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
    String(StringPropertySizes),
    ByteArray(ByteArrayPropertySizes),
    Identifier,
    Boolean,
    Date,
    Object(IndexMap<String, DocumentProperty>),
    Array(ArrayItemType),
    VariableTypeArray(Vec<ArrayItemType>),
    IdentifierWithReference(DocumentPropertyReferenceTarget),
}

impl DocumentPropertyType {
    #[deprecated = "this method is missing required information to create a type. Use TryFrom<&Value> instead."]
    pub fn try_from_name(name: &str) -> Result<Self, DataContractError> {
        match name {
            "u128" => Ok(DocumentPropertyType::U128),
            "i128" => Ok(DocumentPropertyType::I128),
            "u64" => Ok(DocumentPropertyType::U64),
            "i64" | "integer" => Ok(DocumentPropertyType::I64),
            "u32" => Ok(DocumentPropertyType::U32),
            "i32" => Ok(DocumentPropertyType::I32),
            "u16" => Ok(DocumentPropertyType::U16),
            "i16" => Ok(DocumentPropertyType::I16),
            "u8" => Ok(DocumentPropertyType::U8),
            "i8" => Ok(DocumentPropertyType::I8),
            "f64" | "number" => Ok(DocumentPropertyType::F64),
            "boolean" => Ok(DocumentPropertyType::Boolean),
            "date" => Ok(DocumentPropertyType::Date),
            "identifier" => Ok(DocumentPropertyType::Identifier),
            "string" => Ok(DocumentPropertyType::String(StringPropertySizes {
                min_length: None,
                max_length: None,
            })),
            "byteArray" => Ok(DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: None,
                max_size: None,
            })),
            "object" => Ok(DocumentPropertyType::Object(IndexMap::new())),
            "array" => Err(DataContractError::ValueWrongType(
                "array type needs to specify the inner type".to_string(),
            )),
            "variableTypeArray" => Ok(DocumentPropertyType::VariableTypeArray(Vec::new())),
            name => Err(DataContractError::ValueWrongType(format!(
                "invalid type {}",
                name
            ))),
        }
    }

    pub fn name(&self) -> String {
        match self {
            DocumentPropertyType::U128 => "u128".to_string(),
            DocumentPropertyType::I128 => "i128".to_string(),
            DocumentPropertyType::U64 => "u64".to_string(),
            DocumentPropertyType::I64 => "i64".to_string(),
            DocumentPropertyType::U32 => "u32".to_string(),
            DocumentPropertyType::I32 => "i32".to_string(),
            DocumentPropertyType::U16 => "u16".to_string(),
            DocumentPropertyType::I16 => "i16".to_string(),
            DocumentPropertyType::U8 => "u8".to_string(),
            DocumentPropertyType::I8 => "i8".to_string(),
            DocumentPropertyType::F64 => "f64".to_string(),
            DocumentPropertyType::String(_) => "string".to_string(),
            DocumentPropertyType::ByteArray(_) => "byteArray".to_string(),
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                "identifier".to_string()
            }
            DocumentPropertyType::Boolean => "boolean".to_string(),
            DocumentPropertyType::Date => "date".to_string(),
            DocumentPropertyType::Object(_) => "object".to_string(),
            DocumentPropertyType::Array(_) => "array".to_string(),
            DocumentPropertyType::VariableTypeArray(_) => "variableTypeArray".to_string(),
        }
    }

    pub fn min_size(&self) -> Option<u16> {
        match self {
            DocumentPropertyType::U128 => Some(16),
            DocumentPropertyType::I128 => Some(16),
            DocumentPropertyType::U64 => Some(8),
            DocumentPropertyType::I64 => Some(8),
            DocumentPropertyType::U32 => Some(4),
            DocumentPropertyType::I32 => Some(4),
            DocumentPropertyType::U16 => Some(2),
            DocumentPropertyType::I16 => Some(2),
            DocumentPropertyType::U8 => Some(1),
            DocumentPropertyType::I8 => Some(1),
            DocumentPropertyType::F64 => Some(8),
            DocumentPropertyType::String(sizes) => match sizes.min_length {
                None => Some(0),
                Some(size) => Some(size),
            },
            DocumentPropertyType::ByteArray(sizes) => match sizes.min_size {
                None => Some(0),
                Some(size) => Some(size),
            },
            DocumentPropertyType::Boolean => Some(1),
            DocumentPropertyType::Date => Some(8),
            DocumentPropertyType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.property_type.min_size())
                .sum(),
            DocumentPropertyType::Array(_) => None,
            DocumentPropertyType::VariableTypeArray(_) => None,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Some(32)
            }
        }
    }

    pub fn min_byte_size(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u16>, ProtocolError> {
        match self {
            DocumentPropertyType::U128 => Ok(Some(16)),
            DocumentPropertyType::I128 => Ok(Some(16)),
            DocumentPropertyType::U64 => Ok(Some(8)),
            DocumentPropertyType::I64 => Ok(Some(8)),
            DocumentPropertyType::U32 => Ok(Some(4)),
            DocumentPropertyType::I32 => Ok(Some(4)),
            DocumentPropertyType::U16 => Ok(Some(2)),
            DocumentPropertyType::I16 => Ok(Some(2)),
            DocumentPropertyType::U8 => Ok(Some(1)),
            DocumentPropertyType::I8 => Ok(Some(1)),
            DocumentPropertyType::F64 => Ok(Some(8)),
            DocumentPropertyType::String(sizes) => match sizes.min_length {
                None => Ok(Some(0)),
                Some(size) => {
                    if platform_version.protocol_version > 8 {
                        match size.checked_mul(4) {
                            Some(mul) => Ok(Some(mul)),
                            None => Err(ProtocolError::Overflow("min_byte_size overflow")),
                        }
                    } else {
                        Ok(Some(size.wrapping_mul(4)))
                    }
                }
            },
            DocumentPropertyType::ByteArray(sizes) => match sizes.min_size {
                None => Ok(Some(0)),
                Some(size) => Ok(Some(size)),
            },
            DocumentPropertyType::Boolean => Ok(Some(1)),
            DocumentPropertyType::Date => Ok(Some(8)),
            DocumentPropertyType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.property_type.min_byte_size(platform_version))
                .sum(),
            DocumentPropertyType::Array(_) => Ok(None),
            DocumentPropertyType::VariableTypeArray(_) => Ok(None),
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Ok(Some(32))
            }
        }
    }

    pub fn max_byte_size(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u16>, ProtocolError> {
        match self {
            DocumentPropertyType::U128 => Ok(Some(16)),
            DocumentPropertyType::I128 => Ok(Some(16)),
            DocumentPropertyType::U64 => Ok(Some(8)),
            DocumentPropertyType::I64 => Ok(Some(8)),
            DocumentPropertyType::U32 => Ok(Some(4)),
            DocumentPropertyType::I32 => Ok(Some(4)),
            DocumentPropertyType::U16 => Ok(Some(2)),
            DocumentPropertyType::I16 => Ok(Some(2)),
            DocumentPropertyType::U8 => Ok(Some(1)),
            DocumentPropertyType::I8 => Ok(Some(1)),
            DocumentPropertyType::F64 => Ok(Some(8)),
            DocumentPropertyType::String(sizes) => match sizes.max_length {
                None => Ok(Some(u16::MAX)),
                Some(size) => {
                    if platform_version.protocol_version > 8 {
                        match size.checked_mul(4) {
                            Some(mul) => Ok(Some(mul)),
                            None => Err(ProtocolError::Overflow("max_byte_size overflow")),
                        }
                    } else {
                        Ok(Some(size.wrapping_mul(4)))
                    }
                }
            },
            DocumentPropertyType::ByteArray(sizes) => match sizes.max_size {
                None => Ok(Some(u16::MAX)),
                Some(size) => Ok(Some(size)),
            },
            DocumentPropertyType::Boolean => Ok(Some(1)),
            DocumentPropertyType::Date => Ok(Some(8)),
            DocumentPropertyType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.property_type.max_byte_size(platform_version))
                .sum(),
            DocumentPropertyType::Array(_) => Ok(None),
            DocumentPropertyType::VariableTypeArray(_) => Ok(None),
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Ok(Some(32))
            }
        }
    }

    pub fn max_size(&self) -> Option<u16> {
        match self {
            DocumentPropertyType::U128 => Some(16),
            DocumentPropertyType::I128 => Some(16),
            DocumentPropertyType::U64 => Some(8),
            DocumentPropertyType::I64 => Some(8),
            DocumentPropertyType::U32 => Some(4),
            DocumentPropertyType::I32 => Some(4),
            DocumentPropertyType::U16 => Some(2),
            DocumentPropertyType::I16 => Some(2),
            DocumentPropertyType::U8 => Some(1),
            DocumentPropertyType::I8 => Some(1),
            DocumentPropertyType::F64 => Some(8),
            DocumentPropertyType::String(sizes) => match sizes.max_length {
                None => Some(16383),
                Some(size) => Some(size),
            },
            DocumentPropertyType::ByteArray(sizes) => match sizes.max_size {
                None => Some(u16::MAX),
                Some(size) => Some(size),
            },
            DocumentPropertyType::Boolean => Some(1),
            DocumentPropertyType::Date => Some(8),
            DocumentPropertyType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.property_type.max_size())
                .sum(),
            DocumentPropertyType::Array(_) => None,
            DocumentPropertyType::VariableTypeArray(_) => None,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Some(32)
            }
        }
    }

    /// The middle size rounded down halfway between min and max size
    pub fn middle_size(&self, platform_version: &PlatformVersion) -> Option<u16> {
        let min_size = self.min_size()?;
        let max_size = self.max_size()?;
        if platform_version.protocol_version > 8 {
            Some(((min_size as u32 + max_size as u32) / 2) as u16)
        } else {
            Some(min_size.wrapping_add(max_size) / 2)
        }
    }

    /// The middle size rounded up halfway between min and max size
    pub fn middle_size_ceil(&self, platform_version: &PlatformVersion) -> Option<u16> {
        let min_size = self.min_size()?;
        let max_size = self.max_size()?;
        if platform_version.protocol_version > 8 {
            Some(((min_size as u32 + max_size as u32).div_ceil(2)) as u16)
        } else {
            Some(min_size.wrapping_add(max_size).wrapping_add(1) / 2)
        }
    }

    /// The middle size rounded down halfway between min and max byte size
    pub fn middle_byte_size(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u16>, ProtocolError> {
        let Some(min_size) = self.min_byte_size(platform_version)? else {
            return Ok(None);
        };
        let Some(max_size) = self.max_byte_size(platform_version)? else {
            return Ok(None);
        };
        if platform_version.protocol_version > 8 {
            Ok(Some(((min_size as u32 + max_size as u32) / 2) as u16))
        } else {
            Ok(Some(min_size.wrapping_add(max_size) / 2))
        }
    }

    /// The middle size rounded up halfway between min and max byte size
    pub fn middle_byte_size_ceil(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u16>, ProtocolError> {
        let Some(min_size) = self.min_byte_size(platform_version)? else {
            return Ok(None);
        };
        let Some(max_size) = self.max_byte_size(platform_version)? else {
            return Ok(None);
        };
        if platform_version.protocol_version > 8 {
            Ok(Some(
                ((min_size as u32 + max_size as u32).div_ceil(2)) as u16,
            ))
        } else {
            Ok(Some(min_size.wrapping_add(max_size).wrapping_add(1) / 2))
        }
    }

    pub fn random_size(&self, rng: &mut StdRng) -> u16 {
        let min_size = self.min_size().unwrap_or_default();
        let max_size = self.max_size().unwrap_or_default();
        rng.gen_range(min_size..=max_size)
    }

    pub fn random_value(&self, rng: &mut StdRng) -> Value {
        match self {
            DocumentPropertyType::U128 => Value::U128(rng.gen::<u128>()),
            DocumentPropertyType::I128 => Value::I128(rng.gen::<i128>()),
            DocumentPropertyType::U64 => Value::U64(rng.gen::<u64>()),
            DocumentPropertyType::I64 => Value::I64(rng.gen::<i64>()),
            DocumentPropertyType::U32 => Value::U32(rng.gen::<u32>()),
            DocumentPropertyType::I32 => Value::I32(rng.gen::<i32>()),
            DocumentPropertyType::U16 => Value::U16(rng.gen::<u16>()),
            DocumentPropertyType::I16 => Value::I16(rng.gen::<i16>()),
            DocumentPropertyType::U8 => Value::U8(rng.gen::<u8>()),
            DocumentPropertyType::I8 => Value::I8(rng.gen::<i8>()),
            DocumentPropertyType::F64 => Value::Float(rng.gen::<f64>()),
            DocumentPropertyType::String(_) => {
                let size = self.random_size(rng);
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size as usize)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentPropertyType::ByteArray(_) => {
                let size = self.random_size(rng);
                if self.min_size() == self.max_size() {
                    match size {
                        20 => Value::Bytes20(rng.gen()),
                        32 => Value::Bytes32(rng.gen()),
                        36 => Value::Bytes36(
                            rng.sample_iter(Standard)
                                .take(size as usize)
                                .collect::<Vec<_>>()
                                .try_into()
                                .unwrap(),
                        ),
                        _ => Value::Bytes(rng.sample_iter(Standard).take(size as usize).collect()),
                    }
                } else {
                    Value::Bytes(rng.sample_iter(Standard).take(size as usize).collect())
                }
            }
            DocumentPropertyType::Boolean => Value::Bool(rng.gen::<bool>()),
            DocumentPropertyType::Date => {
                let f: f64 = rng.gen_range(1548910575000.0..1648910575000.0);
                Value::Float(f.round() / 1000.0)
            }
            DocumentPropertyType::Object(sub_fields) => {
                let value_vec = sub_fields
                    .iter()
                    .filter_map(|(string, field_type)| {
                        if field_type.required {
                            Some((
                                Value::Text(string.clone()),
                                field_type.property_type.random_value(rng),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                Value::Map(value_vec)
            }
            DocumentPropertyType::Array(_) => Value::Null,
            DocumentPropertyType::VariableTypeArray(_) => Value::Null,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Value::Identifier(rng.gen())
            }
        }
    }

    pub fn random_sub_filled_value(&self, rng: &mut StdRng) -> Value {
        match self {
            DocumentPropertyType::U128 => Value::U128(rng.gen::<u128>()),
            DocumentPropertyType::I128 => Value::I128(rng.gen::<i128>()),
            DocumentPropertyType::U64 => Value::U64(rng.gen::<u64>()),
            DocumentPropertyType::I64 => Value::I64(rng.gen::<i64>()),
            DocumentPropertyType::U32 => Value::U32(rng.gen::<u32>()),
            DocumentPropertyType::I32 => Value::I32(rng.gen::<i32>()),
            DocumentPropertyType::U16 => Value::U16(rng.gen::<u16>()),
            DocumentPropertyType::I16 => Value::I16(rng.gen::<i16>()),
            DocumentPropertyType::U8 => Value::U8(rng.gen::<u8>()),
            DocumentPropertyType::I8 => Value::I8(rng.gen::<i8>()),
            DocumentPropertyType::F64 => Value::Float(rng.gen::<f64>()),
            DocumentPropertyType::String(_) => {
                let size = self.min_size().unwrap();
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size as usize)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentPropertyType::ByteArray(_) => {
                let size = self.min_size().unwrap();
                Value::Bytes(rng.sample_iter(Standard).take(size as usize).collect())
            }
            DocumentPropertyType::Boolean => Value::Bool(rng.gen::<bool>()),
            DocumentPropertyType::Date => {
                let f: f64 = rng.gen_range(1548910575000.0..1648910575000.0);
                Value::Float(f.round() / 1000.0)
            }
            DocumentPropertyType::Object(sub_fields) => {
                let value_vec = sub_fields
                    .iter()
                    .map(|(string, field_type)| {
                        (
                            Value::Text(string.clone()),
                            field_type.property_type.random_sub_filled_value(rng),
                        )
                    })
                    .collect();
                Value::Map(value_vec)
            }
            DocumentPropertyType::Array(_) => Value::Null,
            DocumentPropertyType::VariableTypeArray(_) => Value::Null,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Value::Identifier(rng.gen())
            }
        }
    }

    pub fn random_filled_value(&self, rng: &mut StdRng) -> Value {
        match self {
            DocumentPropertyType::U128 => Value::U128(rng.gen::<u128>()),
            DocumentPropertyType::I128 => Value::I128(rng.gen::<i128>()),
            DocumentPropertyType::U64 => Value::U64(rng.gen::<u64>()),
            DocumentPropertyType::I64 => Value::I64(rng.gen::<i64>()),
            DocumentPropertyType::U32 => Value::U32(rng.gen::<u32>()),
            DocumentPropertyType::I32 => Value::I32(rng.gen::<i32>()),
            DocumentPropertyType::U16 => Value::U16(rng.gen::<u16>()),
            DocumentPropertyType::I16 => Value::I16(rng.gen::<i16>()),
            DocumentPropertyType::U8 => Value::U8(rng.gen::<u8>()),
            DocumentPropertyType::I8 => Value::I8(rng.gen::<i8>()),
            DocumentPropertyType::F64 => Value::Float(rng.gen::<f64>()),
            DocumentPropertyType::String(_) => {
                let size = self.max_size().unwrap();
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size as usize)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentPropertyType::ByteArray(_) => {
                let size = self.max_size().unwrap();
                Value::Bytes(rng.sample_iter(Standard).take(size as usize).collect())
            }
            DocumentPropertyType::Boolean => Value::Bool(rng.gen::<bool>()),
            DocumentPropertyType::Date => {
                let f: f64 = rng.gen_range(1548910575000.0..1648910575000.0);
                Value::Float(f.round() / 1000.0)
            }
            DocumentPropertyType::Object(sub_fields) => {
                let value_vec = sub_fields
                    .iter()
                    .map(|(string, field_type)| {
                        (
                            Value::Text(string.clone()),
                            field_type.property_type.random_filled_value(rng),
                        )
                    })
                    .collect();
                Value::Map(value_vec)
            }
            DocumentPropertyType::Array(_) => Value::Null,
            DocumentPropertyType::VariableTypeArray(_) => Value::Null,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Value::Identifier(rng.gen())
            }
        }
    }

    fn read_varint_value(buf: &mut BufReader<&[u8]>) -> Result<Vec<u8>, DataContractError> {
        let bytes: usize = buf.read_varint().map_err(|_| {
            DataContractError::CorruptedSerialization(
                "error reading varint length from serialized document".to_string(),
            )
        })?;
        if bytes == 0 {
            Ok(vec![])
        } else {
            let mut value: Vec<u8> = vec![0u8; bytes];
            buf.read_exact(&mut value).map_err(|_| {
                DataContractError::CorruptedSerialization(format!(
                    "error reading varint of length {} from serialized document",
                    bytes
                ))
            })?;
            Ok(value)
        }
    }

    /// Reads an optional value from the buffer
    /// Returns an optional value, as well as a boolean to indicate if we have finished the buffer
    pub fn read_optionally_from(
        &self,
        buf: &mut BufReader<&[u8]>,
        required: bool,
    ) -> Result<(Option<Value>, bool), DataContractError> {
        if !required {
            let marker = buf.read_u8().ok();
            match marker {
                None => return Ok((None, true)), // we have no more data
                Some(0) => return Ok((None, false)),
                _ => {}
            }
        }
        match self {
            DocumentPropertyType::U128 => {
                let value = buf.read_u128::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading u128 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::U128(value)), false))
            }
            DocumentPropertyType::I128 => {
                let value = buf.read_i128::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading i128 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I128(value)), false))
            }
            DocumentPropertyType::U64 => {
                let value = buf.read_u64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading u64 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::U64(value)), false))
            }
            DocumentPropertyType::I64 => {
                let value = buf.read_i64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading i64 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I64(value)), false))
            }
            DocumentPropertyType::U32 => {
                let value = buf.read_u32::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading u32 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::U32(value)), false))
            }
            DocumentPropertyType::I32 => {
                let value = buf.read_i32::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading i32 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I32(value)), false))
            }
            DocumentPropertyType::U16 => {
                let value = buf.read_u16::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading u16 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::U16(value)), false))
            }
            DocumentPropertyType::I16 => {
                let value = buf.read_i16::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading i16 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I16(value)), false))
            }
            DocumentPropertyType::U8 => {
                let value = buf.read_u8().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading u8 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::U8(value)), false))
            }
            DocumentPropertyType::I8 => {
                let value = buf.read_i8().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading i8 from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I8(value)), false))
            }
            DocumentPropertyType::String(_) => {
                let bytes = Self::read_varint_value(buf)?;
                let string = String::from_utf8(bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading string from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::Text(string)), false))
            }
            DocumentPropertyType::Date | DocumentPropertyType::F64 => {
                let date = buf.read_f64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading date/number from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::Float(date)), false))
            }
            DocumentPropertyType::Boolean => {
                let value = buf.read_u8().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading bool from serialized document".to_string(),
                    )
                })?;
                match value {
                    0 => Ok((Some(Value::Bool(false)), false)),
                    _ => Ok((Some(Value::Bool(true)), false)),
                }
            }
            DocumentPropertyType::ByteArray(sizes) => {
                match (sizes.min_size, sizes.max_size) {
                    (Some(min), Some(max)) if min == max => {
                        // if min == max, then we don't need a varint for the length
                        let len = min as usize;
                        let mut bytes = vec![0; len];
                        buf.read_exact(&mut bytes).map_err(|_| {
                            DataContractError::DecodingContractError(DecodingError::new(format!(
                                "expected to read {} bytes (min size for byte array)",
                                len
                            )))
                        })?;
                        // To save space we use predefined types for most popular blob sizes
                        // so we don't need to store the size of the blob
                        match bytes.len() {
                            32 => Ok((Some(Value::Bytes32(bytes.try_into().unwrap())), false)),
                            20 => Ok((Some(Value::Bytes20(bytes.try_into().unwrap())), false)),
                            36 => Ok((Some(Value::Bytes36(bytes.try_into().unwrap())), false)),
                            _ => Ok((Some(Value::Bytes(bytes)), false)),
                        }
                    }
                    _ => {
                        let bytes = Self::read_varint_value(buf)?;

                        Ok((Some(Value::Bytes(bytes)), false))
                    }
                }
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                let mut id = [0; 32];
                buf.read_exact(&mut id).map_err(|_| {
                    DataContractError::DecodingContractError(DecodingError::new(
                        "expected to read 32 bytes (identifier)".to_string(),
                    ))
                })?;
                //dbg!(hex::encode(&id));
                Ok((Some(Value::Identifier(id)), false))
            }

            DocumentPropertyType::Object(inner_fields) => {
                let object_byte_len: usize = buf.read_varint().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading varint of object length".to_string(),
                    )
                })?;
                let mut object_bytes = vec![0u8; object_byte_len];
                buf.read_exact(&mut object_bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading object bytes".to_string(),
                    )
                })?;
                // Wrap the bytes in a BufReader
                let mut object_buf_reader = BufReader::new(&object_bytes[..]);
                let mut finished_buffer = false;
                let values = inner_fields
                    .iter()
                    .filter_map(|(key, field)| {
                        if finished_buffer {
                            return if field.required {
                                Some(Err(DataContractError::CorruptedSerialization(
                                    "required field after finished buffer in object".to_string(),
                                )))
                            } else {
                                None
                            };
                        }

                        let read_value = field
                            .property_type
                            .read_optionally_from(&mut object_buf_reader, field.required);

                        match read_value {
                            Ok(read_value) => {
                                finished_buffer |= read_value.1;
                                read_value
                                    .0
                                    .map(|read_value| Ok((Value::Text(key.clone()), read_value)))
                            }
                            Err(e) => Some(Err(e)),
                        }
                    })
                    .collect::<Result<Vec<(Value, Value)>, DataContractError>>()?;
                if values.is_empty() {
                    Ok((None, false))
                } else {
                    Ok((Some(Value::Map(values)), false))
                }
            }
            DocumentPropertyType::Array(_array_field_type) => Err(DataContractError::Unsupported(
                "serialization of arrays not yet supported".to_string(),
            )),
            DocumentPropertyType::VariableTypeArray(_) => Err(DataContractError::Unsupported(
                "serialization of variable type arrays not yet supported".to_string(),
            )),
        }
    }

    pub fn encode_value_with_size(
        &self,
        value: Value,
        required: bool,
    ) -> Result<Vec<u8>, ProtocolError> {
        if value.is_null() {
            return Ok(vec![]);
        }
        match self {
            DocumentPropertyType::String(_) => {
                if let Value::Text(value) = value {
                    let vec = value.into_bytes();
                    let mut r_vec = vec.len().encode_var_vec();
                    r_vec.extend(vec);
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error(&value).into())
                }
            }
            DocumentPropertyType::Date | DocumentPropertyType::F64 => {
                let value_as_f64 = value.into_float().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_f64.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    // if the value wasn't required we need to add a byte to prove it existed
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U128 => {
                let value_as_u128: u128 =
                    value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_u128.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::I128 => {
                let value_as_i128: i128 =
                    value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i128.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U64 => {
                let value_as_u64: u64 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_u64.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::I64 => {
                let value_as_i64: i64 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i64.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U32 => {
                let value_as_u32: u32 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_u32.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::I32 => {
                let value_as_i32: i32 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i32.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U16 => {
                let value_as_u16: u16 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_u16.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::I16 => {
                let value_as_i16: i16 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i16.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U8 => {
                let value_as_u8: u8 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_u8.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::I8 => {
                let value_as_i8: i8 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i8.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::ByteArray(_) => {
                let mut bytes = value.into_binary_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                let mut bytes = value.into_identifier_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            DocumentPropertyType::Boolean => {
                let value_as_boolean = value
                    .as_bool()
                    .ok_or_else(|| get_field_type_matching_error(&value))?;
                // 0 means does not exist
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![2]) // 2 is false
                }
            }
            DocumentPropertyType::Object(inner_fields) => {
                if let Value::Map(map) = value {
                    let mut value_map =
                        Value::map_into_btree_string_map(map).map_err(ProtocolError::ValueError)?;
                    let mut r_vec = vec![];
                    inner_fields.iter().try_for_each(|(key, field)| {
                        if let Some(value) = value_map.remove(key) {
                            let mut serialized_value = field
                                .property_type
                                .encode_value_with_size(value, field.required)?;
                            r_vec.append(&mut serialized_value);
                            Ok(())
                        } else if field.required {
                            Err(ProtocolError::DataContractError(
                                DataContractError::MissingRequiredKey(
                                    "a required field is not present".to_string(),
                                ),
                            ))
                        } else {
                            // We don't have something that wasn't required
                            r_vec.push(0);
                            Ok(())
                        }
                    })?;
                    let mut len_prepended_vec = r_vec.len().encode_var_vec();
                    len_prepended_vec.append(&mut r_vec);
                    Ok(len_prepended_vec)
                } else {
                    Err(get_field_type_matching_error(&value).into())
                }
            }
            DocumentPropertyType::Array(array_field_type) => {
                if let Value::Array(array) = value {
                    let mut r_vec = array.len().encode_var_vec();

                    array.into_iter().try_for_each(|value| {
                        let mut serialized_value =
                            array_field_type.encode_value_with_size(value)?;
                        r_vec.append(&mut serialized_value);
                        Ok::<(), ProtocolError>(())
                    })?;
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error(&value).into())
                }
            }
            DocumentPropertyType::VariableTypeArray(_) => Err(ProtocolError::DataContractError(
                DataContractError::Unsupported(
                    "serialization of variable type arrays not yet supported".to_string(),
                ),
            )),
        }
    }

    pub fn encode_value_ref_with_size(
        &self,
        value: &Value,
        required: bool,
    ) -> Result<Vec<u8>, ProtocolError> {
        if value.is_null() {
            return Ok(vec![]);
        }
        match self {
            DocumentPropertyType::String(_) => {
                let value_as_text = value
                    .as_text()
                    .ok_or_else(|| get_field_type_matching_error(value))?;
                let vec = value_as_text.as_bytes().to_vec();
                let mut r_vec = vec.len().encode_var_vec();
                r_vec.extend(vec);
                Ok(r_vec)
            }
            // TODO: Make the same as in https://github.com/dashpay/platform/blob/8d2a9e54d62b77581c44a15a09a2c61864af37d3/packages/rs-dpp/src/document/v0/serialize.rs#L161
            //  it must be u64 BE. Markers are wrong here as well
            DocumentPropertyType::Date => {
                let value_as_f64 = value.to_float().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_f64.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    // if the value wasn't required we need to add a byte to prove it existed
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::U128 => {
                let value_as_u128: u128 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_u128.to_be_bytes().to_vec())
            }
            DocumentPropertyType::I128 => {
                let value_as_i128: i128 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i128.to_be_bytes().to_vec())
            }
            DocumentPropertyType::U64 => {
                let value_as_u64: u64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_u64.to_be_bytes().to_vec())
            }
            DocumentPropertyType::I64 => {
                let value_as_i64: i64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i64.to_be_bytes().to_vec())
            }
            DocumentPropertyType::U32 => {
                let value_as_u32: u32 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_u32.to_be_bytes().to_vec())
            }
            DocumentPropertyType::I32 => {
                let value_as_i32: i32 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i32.to_be_bytes().to_vec())
            }
            DocumentPropertyType::U16 => {
                let value_as_u16: u16 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_u16.to_be_bytes().to_vec())
            }
            DocumentPropertyType::I16 => {
                let value_as_i16: i16 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i16.to_be_bytes().to_vec())
            }
            DocumentPropertyType::U8 => {
                let value_as_u8: u8 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_u8.to_be_bytes().to_vec())
            }
            DocumentPropertyType::I8 => {
                let value_as_i8: i8 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i8.to_be_bytes().to_vec())
            }
            DocumentPropertyType::F64 => {
                let value_as_f64 = value.to_float().map_err(ProtocolError::ValueError)?;
                Ok(value_as_f64.to_be_bytes().to_vec())
            }
            DocumentPropertyType::ByteArray(sizes) => match (sizes.min_size, sizes.max_size) {
                (Some(min), Some(max)) if min == max => Ok(value.to_binary_bytes()?),
                _ => {
                    let mut bytes = value.to_binary_bytes()?;

                    let mut r_vec = bytes.len().encode_var_vec();
                    r_vec.append(&mut bytes);
                    Ok(r_vec)
                }
            },
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Ok(value.to_identifier_bytes()?)
            }
            DocumentPropertyType::Boolean => {
                let value_as_boolean = value
                    .as_bool()
                    .ok_or_else(|| get_field_type_matching_error(value))?;
                // 0 means does not exist
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![0]) // 0 is false
                }
            }
            DocumentPropertyType::Object(inner_fields) => {
                let Some(value_map) = value.as_map() else {
                    return Err(get_field_type_matching_error(value).into());
                };
                let value_map = Value::map_ref_into_btree_string_map(value_map)?;
                let mut r_vec = vec![];
                inner_fields.iter().try_for_each(|(key, field)| {
                    if let Some(value) = value_map.get(key) {
                        if !field.required {
                            r_vec.push(1);
                        }
                        let value = field
                            .property_type
                            .encode_value_ref_with_size(value, field.required)?;
                        r_vec.extend(value.as_slice());
                        Ok(())
                    } else if field.required {
                        Err(ProtocolError::DataContractError(
                            DataContractError::MissingRequiredKey(
                                "a required field is not present".to_string(),
                            ),
                        ))
                    } else {
                        // We don't have something that wasn't required
                        r_vec.push(0);
                        Ok(())
                    }
                })?;
                let mut len_prepended_vec = r_vec.len().encode_var_vec();
                len_prepended_vec.append(&mut r_vec);
                Ok(len_prepended_vec)
            }
            DocumentPropertyType::Array(array_field_type) => {
                if let Value::Array(array) = value {
                    let mut r_vec = array.len().encode_var_vec();

                    array.iter().try_for_each(|value| {
                        let mut serialized_value =
                            array_field_type.encode_value_ref_with_size(value)?;
                        r_vec.append(&mut serialized_value);
                        Ok::<(), ProtocolError>(())
                    })?;
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error(value).into())
                }
            }

            DocumentPropertyType::VariableTypeArray(_) => Err(ProtocolError::DataContractError(
                DataContractError::Unsupported(
                    "serialization of arrays not yet supported".to_string(),
                ),
            )),
        }
    }

    // Given a field type and a value this function chooses and executes the right encoding method
    pub fn encode_value_for_tree_keys(&self, value: &Value) -> Result<Vec<u8>, ProtocolError> {
        if value.is_null() {
            return Ok(vec![]);
        }
        match self {
            DocumentPropertyType::String(_) => {
                let value_as_text = value
                    .as_text()
                    .ok_or_else(|| get_field_type_matching_error(value))?;
                let vec = value_as_text.as_bytes().to_vec();
                if vec.is_empty() {
                    // we don't want to collide with the definition of an empty string
                    Ok(vec![0])
                } else {
                    Ok(vec)
                }
            }
            DocumentPropertyType::Date => Ok(DocumentPropertyType::encode_date_timestamp(
                value.to_integer().map_err(ProtocolError::ValueError)?,
            )),
            DocumentPropertyType::U128 => {
                let value_as_u128 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_u128(value_as_u128))
            }
            DocumentPropertyType::I128 => {
                let value_as_i128 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_i128(value_as_i128))
            }
            DocumentPropertyType::U64 => {
                let value_as_u64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_u64(value_as_u64))
            }
            DocumentPropertyType::I64 => {
                let value_as_i64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_i64(value_as_i64))
            }
            DocumentPropertyType::U32 => {
                let value_as_u32 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_u32(value_as_u32))
            }
            DocumentPropertyType::I32 => {
                let value_as_i32 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_i32(value_as_i32))
            }
            DocumentPropertyType::U16 => {
                let value_as_u16 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_u16(value_as_u16))
            }
            DocumentPropertyType::I16 => {
                let value_as_i16 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_i16(value_as_i16))
            }
            DocumentPropertyType::U8 => {
                let value_as_u8 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_u8(value_as_u8))
            }
            DocumentPropertyType::I8 => {
                let value_as_i8 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(DocumentPropertyType::encode_i8(value_as_i8))
            }
            DocumentPropertyType::F64 => Ok(Self::encode_float(
                value.to_float().map_err(ProtocolError::ValueError)?,
            )),
            DocumentPropertyType::ByteArray(_) => {
                value.to_binary_bytes().map_err(ProtocolError::ValueError)
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                value
                    .to_identifier_bytes()
                    .map_err(ProtocolError::ValueError)
            }
            DocumentPropertyType::Boolean => {
                let value_as_boolean = value
                    .as_bool()
                    .ok_or_else(|| get_field_type_matching_error(value))?;
                if value_as_boolean {
                    Ok(vec![1])
                } else {
                    Ok(vec![0])
                }
            }
            DocumentPropertyType::Object(_) => Err(ProtocolError::DataContractError(
                DataContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an object".to_string(),
                ),
            )),
            DocumentPropertyType::Array(_) | DocumentPropertyType::VariableTypeArray(_) => {
                Err(ProtocolError::DataContractError(
                    DataContractError::EncodingDataStructureNotSupported(
                        "we should never try encoding an array".to_string(),
                    ),
                ))
            }
        }
    }

    // Given a field type and a Vec<u8> this function chooses and executes the right decoding method
    pub fn decode_value_for_tree_keys(&self, value: &[u8]) -> Result<Value, ProtocolError> {
        if value.is_empty() {
            return Ok(Value::Null);
        }
        match self {
            DocumentPropertyType::String(_) => {
                if value == [0] {
                    // we don't want to collide with the definition of an empty string
                    Ok(Value::Text("".to_string()))
                } else {
                    Ok(Value::Text(String::from_utf8(value.to_vec()).map_err(
                        |_| {
                            ProtocolError::DecodingError(
                                "could not decode utf8 bytes into string".to_string(),
                            )
                        },
                    )?))
                }
            }
            DocumentPropertyType::Date => {
                let timestamp = DocumentPropertyType::decode_date_timestamp(value).ok_or(
                    ProtocolError::DecodingError("could not decode data timestamp".to_string()),
                )?;
                Ok(Value::U64(timestamp))
            }
            DocumentPropertyType::U128 => {
                let integer = DocumentPropertyType::decode_u128(value).ok_or(
                    ProtocolError::DecodingError("could not decode u128".to_string()),
                )?;
                Ok(Value::U128(integer))
            }
            DocumentPropertyType::I128 => {
                let integer = DocumentPropertyType::decode_i128(value).ok_or(
                    ProtocolError::DecodingError("could not decode i128".to_string()),
                )?;
                Ok(Value::I128(integer))
            }
            DocumentPropertyType::U64 => {
                let integer = DocumentPropertyType::decode_u64(value).ok_or(
                    ProtocolError::DecodingError("could not decode u64".to_string()),
                )?;
                Ok(Value::U64(integer))
            }
            DocumentPropertyType::I64 => {
                let integer = DocumentPropertyType::decode_i64(value).ok_or(
                    ProtocolError::DecodingError("could not decode i64".to_string()),
                )?;
                Ok(Value::I64(integer))
            }
            DocumentPropertyType::U32 => {
                let integer = DocumentPropertyType::decode_u32(value).ok_or(
                    ProtocolError::DecodingError("could not decode u32".to_string()),
                )?;
                Ok(Value::U32(integer))
            }
            DocumentPropertyType::I32 => {
                let integer = DocumentPropertyType::decode_i32(value).ok_or(
                    ProtocolError::DecodingError("could not decode i32".to_string()),
                )?;
                Ok(Value::I32(integer))
            }
            DocumentPropertyType::U16 => {
                let integer = DocumentPropertyType::decode_u16(value).ok_or(
                    ProtocolError::DecodingError("could not decode u16".to_string()),
                )?;
                Ok(Value::U16(integer))
            }
            DocumentPropertyType::I16 => {
                let integer = DocumentPropertyType::decode_i16(value).ok_or(
                    ProtocolError::DecodingError("could not decode i16".to_string()),
                )?;
                Ok(Value::I16(integer))
            }
            DocumentPropertyType::U8 => {
                let integer = DocumentPropertyType::decode_u8(value).ok_or(
                    ProtocolError::DecodingError("could not decode u8".to_string()),
                )?;
                Ok(Value::U8(integer))
            }
            DocumentPropertyType::I8 => {
                let integer = DocumentPropertyType::decode_i8(value).ok_or(
                    ProtocolError::DecodingError("could not decode i8".to_string()),
                )?;
                Ok(Value::I8(integer))
            }
            DocumentPropertyType::F64 => {
                let float = DocumentPropertyType::decode_float(value).ok_or(
                    ProtocolError::DecodingError("could not decode float".to_string()),
                )?;
                Ok(Value::Float(float))
            }
            DocumentPropertyType::ByteArray(_) => Ok(Value::Bytes(value.to_vec())),
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                let identifier = Identifier::from_bytes(value)?;
                Ok(identifier.into())
            }
            DocumentPropertyType::Boolean => {
                if value == [0] {
                    Ok(Value::Bool(false))
                } else if value == [1] {
                    Ok(Value::Bool(true))
                } else {
                    Err(ProtocolError::DecodingError(
                        "could not decode bool".to_string(),
                    ))
                }
            }
            DocumentPropertyType::Object(_) => Err(ProtocolError::DataContractError(
                DataContractError::EncodingDataStructureNotSupported(
                    "we should never try decoding an object".to_string(),
                ),
            )),
            DocumentPropertyType::Array(_) | DocumentPropertyType::VariableTypeArray(_) => {
                Err(ProtocolError::DataContractError(
                    DataContractError::EncodingDataStructureNotSupported(
                        "we should never try decoding an array".to_string(),
                    ),
                ))
            }
        }
    }

    // Given a field type and a value this function chooses and executes the right encoding method
    pub fn value_from_string(&self, str: &str) -> Result<Value, DataContractError> {
        match self {
            DocumentPropertyType::String(sizes) => {
                if let Some(min) = sizes.min_length {
                    if str.len() < min as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "string is too small".to_string(),
                        ));
                    }
                }
                if let Some(max) = sizes.max_length {
                    if str.len() > max as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "string is too big".to_string(),
                        ));
                    }
                }
                Ok(Value::Text(str.to_string()))
            }
            DocumentPropertyType::U128 => str.parse::<u128>().map(Value::U128).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not a u128 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::I128 => str.parse::<i128>().map(Value::I128).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not an i128 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::U64 => str.parse::<u64>().map(Value::U64).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not a u64 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::I64 => str.parse::<i64>().map(Value::I64).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not an i64 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::U32 => str.parse::<u32>().map(Value::U32).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not a u32 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::I32 => str.parse::<i32>().map(Value::I32).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not an i32 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::U16 => str.parse::<u16>().map(Value::U16).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not a u16 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::I16 => str.parse::<i16>().map(Value::I16).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not an i16 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::U8 => str.parse::<u8>().map(Value::U8).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not a u8 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::I8 => str.parse::<i8>().map(Value::I8).map_err(|_| {
                DataContractError::ValueWrongType(
                    "value is not an i8 integer from string".to_string(),
                )
            }),
            DocumentPropertyType::F64 | DocumentPropertyType::Date => {
                str.parse::<f64>().map(Value::Float).map_err(|_| {
                    DataContractError::ValueWrongType(
                        "value is not a float from string".to_string(),
                    )
                })
            }
            DocumentPropertyType::ByteArray(sizes) => {
                if let Some(min) = sizes.min_size {
                    if str.len() / 2 < min as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "byte array is too small".to_string(),
                        ));
                    }
                }
                if let Some(max) = sizes.max_size {
                    if str.len() / 2 > max as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "byte array is too big".to_string(),
                        ));
                    }
                }
                Ok(Value::Bytes(hex::decode(str).map_err(|_| {
                    DataContractError::ValueDecodingError("could not parse hex bytes".to_string())
                })?))
            }
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
                Ok(Value::Identifier(
                    Value::Text(str.to_owned())
                        .to_identifier()
                        .map_err(|e| DataContractError::ValueDecodingError(format!("{:?}", e)))?
                        .into_buffer(),
                ))
            }
            DocumentPropertyType::Boolean => {
                if str.to_lowercase().as_str() == "true" {
                    Ok(Value::Bool(true))
                } else if str.to_lowercase().as_str() == "false" {
                    Ok(Value::Bool(false))
                } else {
                    Err(DataContractError::ValueDecodingError(
                        "could not parse a boolean to a value".to_string(),
                    ))
                }
            }
            DocumentPropertyType::Object(_) => {
                Err(DataContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an object".to_string(),
                ))
            }
            DocumentPropertyType::Array(_) | DocumentPropertyType::VariableTypeArray(_) => {
                Err(DataContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an array".to_string(),
                ))
            }
        }
    }

    pub fn encode_date_timestamp(val: TimestampMillis) -> Vec<u8> {
        Self::encode_u64(val)
    }

    pub fn decode_date_timestamp(val: &[u8]) -> Option<TimestampMillis> {
        Self::decode_u64(val)
    }

    pub fn encode_u128(val: u128) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_u128::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    /// Decodes an unsigned integer on 128 bits.
    pub fn decode_u128(val: &[u8]) -> Option<u128> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_u128::<BigEndian>().ok()
    }

    pub fn encode_i128(val: i128) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_i128::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    pub fn decode_i128(val: &[u8]) -> Option<i128> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_i128::<BigEndian>().ok()
    }

    pub fn encode_u64(val: u64) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_u64::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    /// Decodes an unsigned integer on 64 bits.
    pub fn decode_u64(val: &[u8]) -> Option<u64> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_u64::<BigEndian>().ok()
    }

    pub fn encode_i64(val: i64) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_i64::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    pub fn decode_i64(val: &[u8]) -> Option<i64> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_i64::<BigEndian>().ok()
    }

    pub fn encode_u32(val: u32) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_u32::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    /// Decodes an unsigned integer on 32 bits.
    pub fn decode_u32(val: &[u8]) -> Option<u32> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_u32::<BigEndian>().ok()
    }

    pub fn encode_i32(val: i32) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_i32::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    pub fn decode_i32(val: &[u8]) -> Option<i32> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_i32::<BigEndian>().ok()
    }

    pub fn encode_u16(val: u16) -> Vec<u8> {
        //todo this should just be to_be_bytes (and for all unsigned integers)
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_u16::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    /// Decodes an unsigned integer on 16 bits.
    pub fn decode_u16(val: &[u8]) -> Option<u16> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_u16::<BigEndian>().ok()
    }

    pub fn encode_i16(val: i16) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_i16::<BigEndian>(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    pub fn decode_i16(val: &[u8]) -> Option<i16> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_i16::<BigEndian>().ok()
    }

    pub fn encode_u8(val: u8) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_u8(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    /// Decodes an unsigned integer on 8 bits.
    pub fn decode_u8(val: &[u8]) -> Option<u8> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_u8().ok()
    }

    pub fn encode_i8(val: i8) -> Vec<u8> {
        // Positive integers are represented in binary with the signed bit set to 0
        // Negative integers are represented in 2's complement form

        // Encode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut wtr = vec![];
        wtr.write_i8(val).unwrap();

        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        wtr[0] ^= 0b1000_0000;

        wtr
    }

    pub fn decode_i8(val: &[u8]) -> Option<i8> {
        // Flip the sign bit
        // to deal with interaction between the domains
        // 2's complement values have the sign bit set to 1
        // this makes them greater than the positive domain in terms of sort order
        // to fix this, we just flip the sign bit
        // so positive integers have the high bit and negative integers have the low bit
        // the relative order of elements in each domain is still maintained, as the
        // change was uniform across all elements
        let mut val = val.to_vec();
        val[0] ^= 0b1000_0000;

        // Decode the integer in big endian form
        // This ensures that most significant bits are compared first
        // a bigger positive number would be greater than a smaller one
        // and a bigger negative number would be greater than a smaller one
        // maintains sort order for each domain
        let mut rdr = val.as_slice();
        rdr.read_i8().ok()
    }

    pub fn encode_float(val: f64) -> Vec<u8> {
        // Floats are represented based on the  IEEE 754-2008 standard
        // [sign bit] [biased exponent] [mantissa]

        // when comparing floats, the sign bit has the greatest impact
        // any positive number is greater than all negative numbers
        // if the numbers come from the same domain then the exponent is the next factor to consider
        // the exponent gives a sense of how many digits are in the non fractional part of the number
        // for example in base 10, 10 has an exponent of 1 (1.0 * 10^1)
        // while 5000 (5.0 * 10^3) has an exponent of 3
        // for the positive domain, the bigger the exponent the larger the number i.e 5000 > 10
        // for the negative domain, the bigger the exponent the smaller the number i.e -10 > -5000
        // if the exponents are the same, then the mantissa is used to determine the greater number
        // the inverse relationship still holds
        // i.e bigger mantissa (bigger number in positive domain but smaller number in negative domain)

        // There are two things to fix to achieve total sort order
        // 1. Place positive domain above negative domain (i.e flip the sign bit)
        // 2. Exponent and mantissa for a smaller number like -5000 is greater than that of -10
        //    so bit level comparison would say -5000 is greater than -10
        //    we fix this by flipping the exponent and mantissa values, which has the effect of reversing
        //    the order (0000 [smallest] -> 1111 [largest])

        // Encode in big endian form, so most significant bits are compared first
        let mut wtr = vec![];
        wtr.write_f64::<BigEndian>(val).unwrap();

        // Check if the value is negative, if it is
        // flip all the bits i.e sign, exponent and mantissa
        if val < 0.0 {
            wtr = wtr.iter().map(|byte| !byte).collect();
        } else {
            // for positive values, just flip the sign bit
            wtr[0] ^= 0b1000_0000;
        }

        wtr
    }

    /// Decodes a float on 64 bits.
    pub fn decode_float(encoded: &[u8]) -> Option<f64> {
        // Check if the value is negative by looking at the original sign bit
        let is_negative = (encoded[0] & 0b1000_0000) == 0;

        // Create a mutable copy of the encoded vector to apply transformations
        let mut wtr = encoded.to_vec();

        if is_negative {
            // For originally negative values, flip all the bits back
            wtr = wtr.iter().map(|byte| !byte).collect();
        } else {
            // For originally positive values, just flip the sign bit back
            wtr[0] ^= 0b1000_0000;
        }

        // Read the float value from the transformed vector
        let mut cursor = Cursor::new(wtr);
        cursor.read_f64::<BigEndian>().ok()
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            DocumentPropertyType::I8
                | DocumentPropertyType::I16
                | DocumentPropertyType::I32
                | DocumentPropertyType::I64
                | DocumentPropertyType::U8
                | DocumentPropertyType::U16
                | DocumentPropertyType::U32
                | DocumentPropertyType::U64
        )
    }

    pub fn sanitize_value_mut(&self, value: &mut Value) {
        match (self, value.clone()) {
            // Convert hex or base64 strings to byte arrays for ByteArray fields
            (DocumentPropertyType::ByteArray(property_sizes), Value::Text(str_value)) => {
                // Try to decode the string
                let decoded_bytes = if let Ok(bytes) = hex::decode(&str_value) {
                    Some(bytes)
                } else {
                    // If hex fails, try base64 decoding
                    use base64::{engine::general_purpose, Engine as _};
                    general_purpose::STANDARD.decode(str_value).ok()
                };

                if let Some(bytes) = decoded_bytes {
                    let byte_len = bytes.len();

                    // Check if the decoded bytes meet the size constraints
                    let size_ok = match (property_sizes.min_size, property_sizes.max_size) {
                        (Some(min), Some(max)) => {
                            byte_len >= min as usize && byte_len <= max as usize
                        }
                        (Some(min), None) => byte_len >= min as usize,
                        (None, Some(max)) => byte_len <= max as usize,
                        (None, None) => true,
                    };

                    if size_ok {
                        // Use specific byte array types for exact sizes
                        match bytes.len() {
                            20 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes20(arr);
                                }
                            }
                            32 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes32(arr);
                                }
                            }
                            36 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes36(arr);
                                }
                            }
                            _ => {
                                *value = Value::Bytes(bytes);
                            }
                        }
                    }
                    // If size constraints are not met, leave the value as is
                }
                // If decoding fails, leave the value as is (validation will catch it later)
            }

            // Normalize an array of integers to bytes for ByteArray fields. A
            // binary property re-hydrated through a schemaless JSON layer (e.g. an
            // edited-and-replaced cached document) arrives as a plain array of
            // numbers rather than Value::Bytes; convert it here, on the client
            // build path, so the strict binary serializer receives Value::Bytes.
            // (The block-processing serialize path never sanitizes, so this does
            // not change which state transitions are accepted.)
            (DocumentPropertyType::ByteArray(property_sizes), Value::Array(array)) => {
                let decoded: Result<Vec<u8>, _> =
                    array.iter().map(|byte| byte.to_integer::<u8>()).collect();

                if let Ok(bytes) = decoded {
                    let byte_len = bytes.len();

                    let size_ok = match (property_sizes.min_size, property_sizes.max_size) {
                        (Some(min), Some(max)) => {
                            byte_len >= min as usize && byte_len <= max as usize
                        }
                        (Some(min), None) => byte_len >= min as usize,
                        (None, Some(max)) => byte_len <= max as usize,
                        (None, None) => true,
                    };

                    if size_ok {
                        match bytes.len() {
                            20 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes20(arr);
                                }
                            }
                            32 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes32(arr);
                                }
                            }
                            36 => {
                                if let Ok(arr) = bytes.try_into() {
                                    *value = Value::Bytes36(arr);
                                }
                            }
                            _ => {
                                *value = Value::Bytes(bytes);
                            }
                        }
                    }
                    // If size constraints are not met, leave the value as is.
                }
                // If any element is not a 0..=255 integer, leave the value as is
                // (validation will reject it later).
            }

            // Convert hex or base58 strings to identifiers for Identifier fields
            (
                DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_),
                Value::Text(str_value),
            ) => {
                // First try base58 decoding (most common for identifiers)
                if let Ok(id) = Identifier::from_string_unknown_encoding(&str_value) {
                    *value = Value::Identifier(id.into_buffer());
                }
                // If both conversions fail, leave the value as is (validation will catch it later)
            }

            // Ensure integers are in the correct range for their type
            (DocumentPropertyType::U8, Value::U8(_)) => {} // Already correct
            (DocumentPropertyType::U8, Value::U16(n)) if n <= u8::MAX as u16 => {
                *value = Value::U8(n as u8);
            }
            (DocumentPropertyType::U8, Value::U32(n)) if n <= u8::MAX as u32 => {
                *value = Value::U8(n as u8);
            }
            (DocumentPropertyType::U8, Value::U64(n)) if n <= u8::MAX as u64 => {
                *value = Value::U8(n as u8);
            }
            (DocumentPropertyType::U8, Value::U128(n)) if n <= u8::MAX as u128 => {
                *value = Value::U8(n as u8);
            }

            (DocumentPropertyType::U16, Value::U16(_)) => {} // Already correct
            (DocumentPropertyType::U16, Value::U8(n)) => {
                *value = Value::U16(n as u16);
            }
            (DocumentPropertyType::U16, Value::U32(n)) if n <= u16::MAX as u32 => {
                *value = Value::U16(n as u16);
            }
            (DocumentPropertyType::U16, Value::U64(n)) if n <= u16::MAX as u64 => {
                *value = Value::U16(n as u16);
            }
            (DocumentPropertyType::U16, Value::U128(n)) if n <= u16::MAX as u128 => {
                *value = Value::U16(n as u16);
            }

            (DocumentPropertyType::U32, Value::U32(_)) => {} // Already correct
            (DocumentPropertyType::U32, Value::U8(n)) => {
                *value = Value::U32(n as u32);
            }
            (DocumentPropertyType::U32, Value::U16(n)) => {
                *value = Value::U32(n as u32);
            }
            (DocumentPropertyType::U32, Value::U64(n)) if n <= u32::MAX as u64 => {
                *value = Value::U32(n as u32);
            }
            (DocumentPropertyType::U32, Value::U128(n)) if n <= u32::MAX as u128 => {
                *value = Value::U32(n as u32);
            }

            (DocumentPropertyType::U64, Value::U64(_)) => {} // Already correct
            (DocumentPropertyType::U64, Value::U8(n)) => {
                *value = Value::U64(n as u64);
            }
            (DocumentPropertyType::U64, Value::U16(n)) => {
                *value = Value::U64(n as u64);
            }
            (DocumentPropertyType::U64, Value::U32(n)) => {
                *value = Value::U64(n as u64);
            }
            (DocumentPropertyType::U64, Value::U128(n)) if n <= u64::MAX as u128 => {
                *value = Value::U64(n as u64);
            }

            (DocumentPropertyType::U128, Value::U128(_)) => {} // Already correct
            (DocumentPropertyType::U128, Value::U8(n)) => {
                *value = Value::U128(n as u128);
            }
            (DocumentPropertyType::U128, Value::U16(n)) => {
                *value = Value::U128(n as u128);
            }
            (DocumentPropertyType::U128, Value::U32(n)) => {
                *value = Value::U128(n as u128);
            }
            (DocumentPropertyType::U128, Value::U64(n)) => {
                *value = Value::U128(n as u128);
            }

            // Handle signed integers similarly
            (DocumentPropertyType::I8, Value::I8(_)) => {} // Already correct
            (DocumentPropertyType::I8, Value::I16(n))
                if n >= i8::MIN as i16 && n <= i8::MAX as i16 =>
            {
                *value = Value::I8(n as i8);
            }
            (DocumentPropertyType::I8, Value::I32(n))
                if n >= i8::MIN as i32 && n <= i8::MAX as i32 =>
            {
                *value = Value::I8(n as i8);
            }
            (DocumentPropertyType::I8, Value::I64(n))
                if n >= i8::MIN as i64 && n <= i8::MAX as i64 =>
            {
                *value = Value::I8(n as i8);
            }
            (DocumentPropertyType::I8, Value::I128(n))
                if n >= i8::MIN as i128 && n <= i8::MAX as i128 =>
            {
                *value = Value::I8(n as i8);
            }

            (DocumentPropertyType::I16, Value::I16(_)) => {} // Already correct
            (DocumentPropertyType::I16, Value::I8(n)) => {
                *value = Value::I16(n as i16);
            }
            (DocumentPropertyType::I16, Value::I32(n))
                if n >= i16::MIN as i32 && n <= i16::MAX as i32 =>
            {
                *value = Value::I16(n as i16);
            }
            (DocumentPropertyType::I16, Value::I64(n))
                if n >= i16::MIN as i64 && n <= i16::MAX as i64 =>
            {
                *value = Value::I16(n as i16);
            }
            (DocumentPropertyType::I16, Value::I128(n))
                if n >= i16::MIN as i128 && n <= i16::MAX as i128 =>
            {
                *value = Value::I16(n as i16);
            }

            (DocumentPropertyType::I32, Value::I32(_)) => {} // Already correct
            (DocumentPropertyType::I32, Value::I8(n)) => {
                *value = Value::I32(n as i32);
            }
            (DocumentPropertyType::I32, Value::I16(n)) => {
                *value = Value::I32(n as i32);
            }
            (DocumentPropertyType::I32, Value::I64(n))
                if n >= i32::MIN as i64 && n <= i32::MAX as i64 =>
            {
                *value = Value::I32(n as i32);
            }
            (DocumentPropertyType::I32, Value::I128(n))
                if n >= i32::MIN as i128 && n <= i32::MAX as i128 =>
            {
                *value = Value::I32(n as i32);
            }

            (DocumentPropertyType::I64, Value::I64(_)) => {} // Already correct
            (DocumentPropertyType::I64, Value::I8(n)) => {
                *value = Value::I64(n as i64);
            }
            (DocumentPropertyType::I64, Value::I16(n)) => {
                *value = Value::I64(n as i64);
            }
            (DocumentPropertyType::I64, Value::I32(n)) => {
                *value = Value::I64(n as i64);
            }
            (DocumentPropertyType::I64, Value::I128(n))
                if n >= i64::MIN as i128 && n <= i64::MAX as i128 =>
            {
                *value = Value::I64(n as i64);
            }

            (DocumentPropertyType::I128, Value::I128(_)) => {} // Already correct
            (DocumentPropertyType::I128, Value::I8(n)) => {
                *value = Value::I128(n as i128);
            }
            (DocumentPropertyType::I128, Value::I16(n)) => {
                *value = Value::I128(n as i128);
            }
            (DocumentPropertyType::I128, Value::I32(n)) => {
                *value = Value::I128(n as i128);
            }
            (DocumentPropertyType::I128, Value::I64(n)) => {
                *value = Value::I128(n as i128);
            }

            // Handle Date type - convert integers to date
            (DocumentPropertyType::Date, Value::U64(_)) => {
                // Timestamp is already in the right format (milliseconds since epoch)
                // But we might want to validate it's a reasonable date
                // For now, just leave it as is
            }
            (DocumentPropertyType::Date, Value::I64(timestamp)) if timestamp >= 0 => {
                *value = Value::U64(timestamp as u64);
            }

            // Handle Object type - recursively sanitize nested fields
            (DocumentPropertyType::Object(schema), Value::Map(_)) => {
                if let Value::Map(map) = value {
                    for (key, nested_value) in map.iter_mut() {
                        if let Value::Text(field_name) = key {
                            if let Some(field_property) = schema.get(field_name) {
                                field_property
                                    .property_type
                                    .sanitize_value_mut(nested_value);
                            }
                        }
                    }
                }
            }

            // Handle Array type - sanitize all elements
            (DocumentPropertyType::Array(item_type), Value::Array(_)) => {
                if let Value::Array(items) = value {
                    for item in items.iter_mut() {
                        item_type.sanitize_value_mut(item);
                    }
                }
            }

            // Handle VariableTypeArray - each item can have a different type
            (DocumentPropertyType::VariableTypeArray(item_types), Value::Array(_)) => {
                if let Value::Array(items) = value {
                    for (item, item_type) in items.iter_mut().zip(item_types.iter().cycle()) {
                        item_type.sanitize_value_mut(item);
                    }
                }
            }

            // For all other cases, leave the value as is
            _ => {}
        }
    }

    pub fn try_from_value_map(
        value_map: &BTreeMap<String, &Value>,
        options: &DocumentPropertyTypeParsingOptions,
    ) -> Result<Self, DataContractError> {
        let type_value = value_map.get_str(property_names::TYPE)?;

        let property_type = match type_value {
            "integer" => {
                if options.sized_integer_types {
                    find_integer_type_for_subschema_value(value_map)?
                } else {
                    DocumentPropertyType::I64
                }
            }
            "string" => DocumentPropertyType::String(StringPropertySizes {
                min_length: value_map.get_optional_integer(property_names::MIN_LENGTH)?,
                max_length: value_map.get_optional_integer(property_names::MAX_LENGTH)?,
            }),
            "array" => {
                // Only handling bytearrays for v1
                // Return an error if it is not a byte array
                let Some(is_byte_array) =
                    value_map.get_optional_bool(property_names::BYTE_ARRAY)?
                else {
                    return Err(DataContractError::InvalidContractStructure(
                        "only byte arrays are supported now".to_string(),
                    ));
                };

                if !is_byte_array {
                    return Err(DataContractError::InvalidContractStructure(
                        "byteArray should always be true if defined".to_string(),
                    ));
                }

                match value_map.get_optional_str(property_names::CONTENT_MEDIA_TYPE)? {
                    Some("application/x.dash.dpp.identifier") => DocumentPropertyType::Identifier,
                    Some(_) | None => DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                        min_size: value_map.get_optional_integer(property_names::MIN_ITEMS)?,
                        max_size: value_map.get_optional_integer(property_names::MAX_ITEMS)?,
                    }),
                }
            }
            "object" => Self::Object(Default::default()),
            "boolean" => DocumentPropertyType::Boolean,
            "number" => DocumentPropertyType::F64,
            _ => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "unsupported property type: {}",
                    type_value
                )));
            }
        };

        Ok(property_type)
    }
}

#[derive(Debug, Clone)]
pub struct DocumentPropertyTypeParsingOptions {
    pub sized_integer_types: bool,
}

impl Default for DocumentPropertyTypeParsingOptions {
    fn default() -> Self {
        Self {
            sized_integer_types: true,
        }
    }
}

impl From<&DataContractConfig> for DocumentPropertyTypeParsingOptions {
    fn from(config: &DataContractConfig) -> Self {
        Self {
            sized_integer_types: config.sized_integer_types(),
        }
    }
}

fn get_field_type_matching_error(value: &Value) -> DataContractError {
    DataContractError::ValueWrongType(format!(
        "document field type doesn't match \"{}\" document value",
        value
    ))
}

fn find_integer_type_for_subschema_value(
    value: &BTreeMap<String, &Value>,
) -> Result<DocumentPropertyType, DataContractError> {
    let minimum = value.get_optional_integer::<i64>(property_names::MINIMUM)?;
    let maximum = value.get_optional_integer::<i64>(property_names::MAXIMUM)?;

    let property_type = match (minimum, maximum) {
        (Some(min), Some(max)) => find_integer_type_for_min_and_max_values(min, max),
        (Some(min), None) => {
            if min >= 0 {
                DocumentPropertyType::U64
            } else {
                DocumentPropertyType::I64
            }
        }
        (None, Some(max)) => find_unsigned_integer_type_for_max_value(max),
        (None, None) => {
            // If enum is defined, we can try to figure out type based on minimal and maximal values
            let enum_type = if let Some(enum_values) =
                value.get_optional_inner_value_array::<Vec<_>>(property_names::ENUM)?
            {
                match enum_values
                    .into_iter()
                    .filter_map(|v| v.as_integer())
                    .minmax()
                {
                    itertools::MinMaxResult::MinMax(min, max) => {
                        Some(find_integer_type_for_min_and_max_values(min, max))
                    }
                    itertools::MinMaxResult::OneElement(val) => {
                        Some(find_unsigned_integer_type_for_max_value(val))
                    }
                    _ => None,
                }
            } else {
                None
            };

            if let Some(enum_type) = enum_type {
                enum_type
            } else {
                DocumentPropertyType::I64
            }
        }
    };

    Ok(property_type)
}

fn find_unsigned_integer_type_for_max_value(max_value: i64) -> DocumentPropertyType {
    if max_value <= u8::MAX as i64 {
        DocumentPropertyType::U8
    } else if max_value <= u16::MAX as i64 {
        DocumentPropertyType::U16
    } else if max_value <= u32::MAX as i64 {
        DocumentPropertyType::U32
    } else {
        DocumentPropertyType::U64
    }
}

fn find_integer_type_for_min_and_max_values(min: i64, max: i64) -> DocumentPropertyType {
    if min >= 0 {
        find_unsigned_integer_type_for_max_value(max)
    } else if min >= i8::MIN as i64 && max <= i8::MAX as i64 {
        DocumentPropertyType::I8
    } else if min >= i16::MIN as i64 && max <= i16::MAX as i64 {
        DocumentPropertyType::I16
    } else if min >= i32::MIN as i64 && max <= i32::MAX as i64 {
        DocumentPropertyType::I32
    } else {
        DocumentPropertyType::I64
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use platform_version::version::PlatformVersion;

    // -----------------------------------------------------------------------
    // name() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_name_returns_correct_string_for_all_variants() {
        let cases: Vec<(DocumentPropertyType, &str)> = vec![
            (DocumentPropertyType::U128, "u128"),
            (DocumentPropertyType::I128, "i128"),
            (DocumentPropertyType::U64, "u64"),
            (DocumentPropertyType::I64, "i64"),
            (DocumentPropertyType::U32, "u32"),
            (DocumentPropertyType::I32, "i32"),
            (DocumentPropertyType::U16, "u16"),
            (DocumentPropertyType::I16, "i16"),
            (DocumentPropertyType::U8, "u8"),
            (DocumentPropertyType::I8, "i8"),
            (DocumentPropertyType::F64, "f64"),
            (
                DocumentPropertyType::String(StringPropertySizes {
                    min_length: None,
                    max_length: None,
                }),
                "string",
            ),
            (
                DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                    min_size: None,
                    max_size: None,
                }),
                "byteArray",
            ),
            (DocumentPropertyType::Identifier, "identifier"),
            (DocumentPropertyType::Boolean, "boolean"),
            (DocumentPropertyType::Date, "date"),
            (DocumentPropertyType::Object(IndexMap::new()), "object"),
            (DocumentPropertyType::Array(ArrayItemType::Integer), "array"),
            (
                DocumentPropertyType::VariableTypeArray(vec![]),
                "variableTypeArray",
            ),
        ];
        for (prop_type, expected) in cases {
            assert_eq!(
                prop_type.name(),
                expected,
                "name() mismatch for {:?}",
                prop_type
            );
        }
    }

    // -----------------------------------------------------------------------
    // try_from_name() tests
    // -----------------------------------------------------------------------

    #[test]
    #[allow(deprecated)]
    fn test_try_from_name_known_types() {
        assert_eq!(
            DocumentPropertyType::try_from_name("u128").unwrap(),
            DocumentPropertyType::U128
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("i128").unwrap(),
            DocumentPropertyType::I128
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("u64").unwrap(),
            DocumentPropertyType::U64
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("i64").unwrap(),
            DocumentPropertyType::I64
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("integer").unwrap(),
            DocumentPropertyType::I64
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("u32").unwrap(),
            DocumentPropertyType::U32
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("i32").unwrap(),
            DocumentPropertyType::I32
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("u16").unwrap(),
            DocumentPropertyType::U16
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("i16").unwrap(),
            DocumentPropertyType::I16
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("u8").unwrap(),
            DocumentPropertyType::U8
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("i8").unwrap(),
            DocumentPropertyType::I8
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("f64").unwrap(),
            DocumentPropertyType::F64
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("number").unwrap(),
            DocumentPropertyType::F64
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("boolean").unwrap(),
            DocumentPropertyType::Boolean
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("date").unwrap(),
            DocumentPropertyType::Date
        );
        assert_eq!(
            DocumentPropertyType::try_from_name("identifier").unwrap(),
            DocumentPropertyType::Identifier
        );
        assert!(DocumentPropertyType::try_from_name("string").is_ok());
        assert!(DocumentPropertyType::try_from_name("byteArray").is_ok());
        assert!(DocumentPropertyType::try_from_name("object").is_ok());
    }

    #[test]
    #[allow(deprecated)]
    fn test_try_from_name_array_returns_error() {
        assert!(DocumentPropertyType::try_from_name("array").is_err());
    }

    #[test]
    #[allow(deprecated)]
    fn test_try_from_name_unknown_returns_error() {
        assert!(DocumentPropertyType::try_from_name("unknown_type").is_err());
    }

    // -----------------------------------------------------------------------
    // min_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_min_size_fixed_width_types() {
        assert_eq!(DocumentPropertyType::U128.min_size(), Some(16));
        assert_eq!(DocumentPropertyType::I128.min_size(), Some(16));
        assert_eq!(DocumentPropertyType::U64.min_size(), Some(8));
        assert_eq!(DocumentPropertyType::I64.min_size(), Some(8));
        assert_eq!(DocumentPropertyType::U32.min_size(), Some(4));
        assert_eq!(DocumentPropertyType::I32.min_size(), Some(4));
        assert_eq!(DocumentPropertyType::U16.min_size(), Some(2));
        assert_eq!(DocumentPropertyType::I16.min_size(), Some(2));
        assert_eq!(DocumentPropertyType::U8.min_size(), Some(1));
        assert_eq!(DocumentPropertyType::I8.min_size(), Some(1));
        assert_eq!(DocumentPropertyType::F64.min_size(), Some(8));
        assert_eq!(DocumentPropertyType::Boolean.min_size(), Some(1));
        assert_eq!(DocumentPropertyType::Date.min_size(), Some(8));
        assert_eq!(DocumentPropertyType::Identifier.min_size(), Some(32));
    }

    #[test]
    fn test_min_size_string_with_and_without_min_length() {
        let no_min = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        assert_eq!(no_min.min_size(), Some(0));

        let with_min = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(5),
            max_length: None,
        });
        assert_eq!(with_min.min_size(), Some(5));
    }

    #[test]
    fn test_min_size_byte_array_with_and_without_min() {
        let no_min = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        assert_eq!(no_min.min_size(), Some(0));

        let with_min = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(10),
            max_size: None,
        });
        assert_eq!(with_min.min_size(), Some(10));
    }

    #[test]
    fn test_min_size_array_and_variable_type_array_return_none() {
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.min_size(), None);

        let vta = DocumentPropertyType::VariableTypeArray(vec![]);
        assert_eq!(vta.min_size(), None);
    }

    #[test]
    fn test_min_size_object_sums_sub_field_sizes() {
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "field1".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
            },
        );
        sub_fields.insert(
            "field2".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U64,
                required: true,
                transient: false,
            },
        );
        let obj = DocumentPropertyType::Object(sub_fields);
        assert_eq!(obj.min_size(), Some(12)); // 4 + 8
    }

    // -----------------------------------------------------------------------
    // max_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_max_size_fixed_width_types() {
        assert_eq!(DocumentPropertyType::U128.max_size(), Some(16));
        assert_eq!(DocumentPropertyType::I128.max_size(), Some(16));
        assert_eq!(DocumentPropertyType::U64.max_size(), Some(8));
        assert_eq!(DocumentPropertyType::I64.max_size(), Some(8));
        assert_eq!(DocumentPropertyType::U32.max_size(), Some(4));
        assert_eq!(DocumentPropertyType::I32.max_size(), Some(4));
        assert_eq!(DocumentPropertyType::U16.max_size(), Some(2));
        assert_eq!(DocumentPropertyType::I16.max_size(), Some(2));
        assert_eq!(DocumentPropertyType::U8.max_size(), Some(1));
        assert_eq!(DocumentPropertyType::I8.max_size(), Some(1));
        assert_eq!(DocumentPropertyType::F64.max_size(), Some(8));
        assert_eq!(DocumentPropertyType::Boolean.max_size(), Some(1));
        assert_eq!(DocumentPropertyType::Date.max_size(), Some(8));
        assert_eq!(DocumentPropertyType::Identifier.max_size(), Some(32));
    }

    #[test]
    fn test_max_size_string_defaults_and_explicit() {
        let no_max = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        assert_eq!(no_max.max_size(), Some(16383));

        let with_max = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(100),
        });
        assert_eq!(with_max.max_size(), Some(100));
    }

    #[test]
    fn test_max_size_byte_array_defaults_and_explicit() {
        let no_max = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        assert_eq!(no_max.max_size(), Some(u16::MAX));

        let with_max = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: Some(256),
        });
        assert_eq!(with_max.max_size(), Some(256));
    }

    #[test]
    fn test_max_size_array_and_variable_type_array_return_none() {
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.max_size(), None);
        let vta = DocumentPropertyType::VariableTypeArray(vec![]);
        assert_eq!(vta.max_size(), None);
    }

    // -----------------------------------------------------------------------
    // min_byte_size() / max_byte_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_min_byte_size_fixed_width() {
        let pv = PlatformVersion::latest();
        assert_eq!(
            DocumentPropertyType::U128.min_byte_size(pv).unwrap(),
            Some(16)
        );
        assert_eq!(
            DocumentPropertyType::I128.min_byte_size(pv).unwrap(),
            Some(16)
        );
        assert_eq!(
            DocumentPropertyType::U64.min_byte_size(pv).unwrap(),
            Some(8)
        );
        assert_eq!(
            DocumentPropertyType::I64.min_byte_size(pv).unwrap(),
            Some(8)
        );
        assert_eq!(
            DocumentPropertyType::U32.min_byte_size(pv).unwrap(),
            Some(4)
        );
        assert_eq!(
            DocumentPropertyType::I32.min_byte_size(pv).unwrap(),
            Some(4)
        );
        assert_eq!(
            DocumentPropertyType::U16.min_byte_size(pv).unwrap(),
            Some(2)
        );
        assert_eq!(
            DocumentPropertyType::I16.min_byte_size(pv).unwrap(),
            Some(2)
        );
        assert_eq!(DocumentPropertyType::U8.min_byte_size(pv).unwrap(), Some(1));
        assert_eq!(DocumentPropertyType::I8.min_byte_size(pv).unwrap(), Some(1));
        assert_eq!(
            DocumentPropertyType::F64.min_byte_size(pv).unwrap(),
            Some(8)
        );
        assert_eq!(
            DocumentPropertyType::Boolean.min_byte_size(pv).unwrap(),
            Some(1)
        );
        assert_eq!(
            DocumentPropertyType::Date.min_byte_size(pv).unwrap(),
            Some(8)
        );
        assert_eq!(
            DocumentPropertyType::Identifier.min_byte_size(pv).unwrap(),
            Some(32)
        );
    }

    #[test]
    fn test_min_byte_size_string_multiplied_by_4() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(10),
            max_length: None,
        });
        // protocol version > 8 => checked_mul(4)
        assert_eq!(s.min_byte_size(pv).unwrap(), Some(40));
    }

    #[test]
    fn test_min_byte_size_string_no_min() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        assert_eq!(s.min_byte_size(pv).unwrap(), Some(0));
    }

    #[test]
    fn test_max_byte_size_string_multiplied_by_4() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(100),
        });
        assert_eq!(s.max_byte_size(pv).unwrap(), Some(400));
    }

    #[test]
    fn test_max_byte_size_string_no_max() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        assert_eq!(s.max_byte_size(pv).unwrap(), Some(u16::MAX));
    }

    #[test]
    fn test_min_byte_size_byte_array_with_min() {
        let pv = PlatformVersion::latest();
        let ba = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(20),
            max_size: Some(100),
        });
        assert_eq!(ba.min_byte_size(pv).unwrap(), Some(20));
    }

    #[test]
    fn test_max_byte_size_byte_array_with_max() {
        let pv = PlatformVersion::latest();
        let ba = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: Some(200),
        });
        assert_eq!(ba.max_byte_size(pv).unwrap(), Some(200));
    }

    #[test]
    fn test_max_byte_size_byte_array_no_max() {
        let pv = PlatformVersion::latest();
        let ba = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        assert_eq!(ba.max_byte_size(pv).unwrap(), Some(u16::MAX));
    }

    #[test]
    fn test_min_byte_size_array_returns_none() {
        let pv = PlatformVersion::latest();
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.min_byte_size(pv).unwrap(), None);
    }

    #[test]
    fn test_max_byte_size_array_returns_none() {
        let pv = PlatformVersion::latest();
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.max_byte_size(pv).unwrap(), None);
    }

    // -----------------------------------------------------------------------
    // middle_size() / middle_size_ceil() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_middle_size_fixed_type() {
        let pv = PlatformVersion::latest();
        // U32: min=4, max=4 => middle = (4+4)/2 = 4
        assert_eq!(DocumentPropertyType::U32.middle_size(pv), Some(4));
    }

    #[test]
    fn test_middle_size_string() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(0),
            max_length: Some(100),
        });
        // min_size=0, max_size=100 => (0+100)/2 = 50
        assert_eq!(s.middle_size(pv), Some(50));
    }

    #[test]
    fn test_middle_size_ceil_string() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(0),
            max_length: Some(101),
        });
        // min_size=0, max_size=101 => ceil((0+101)/2) = 51
        assert_eq!(s.middle_size_ceil(pv), Some(51));
    }

    #[test]
    fn test_middle_size_returns_none_for_array() {
        let pv = PlatformVersion::latest();
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.middle_size(pv), None);
    }

    // -----------------------------------------------------------------------
    // middle_byte_size() / middle_byte_size_ceil() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_middle_byte_size_fixed_type() {
        let pv = PlatformVersion::latest();
        assert_eq!(
            DocumentPropertyType::U64.middle_byte_size(pv).unwrap(),
            Some(8)
        );
    }

    #[test]
    fn test_middle_byte_size_returns_none_for_array() {
        let pv = PlatformVersion::latest();
        let arr = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert_eq!(arr.middle_byte_size(pv).unwrap(), None);
    }

    #[test]
    fn test_middle_byte_size_ceil_string() {
        let pv = PlatformVersion::latest();
        let s = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(1),
            max_length: Some(10),
        });
        // min_byte_size = 1*4 = 4, max_byte_size = 10*4 = 40
        // ceil((4+40)/2) = 22
        assert_eq!(s.middle_byte_size_ceil(pv).unwrap(), Some(22));
    }

    // -----------------------------------------------------------------------
    // is_integer() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_integer_returns_true_for_integer_types() {
        assert!(DocumentPropertyType::I8.is_integer());
        assert!(DocumentPropertyType::I16.is_integer());
        assert!(DocumentPropertyType::I32.is_integer());
        assert!(DocumentPropertyType::I64.is_integer());
        assert!(DocumentPropertyType::U8.is_integer());
        assert!(DocumentPropertyType::U16.is_integer());
        assert!(DocumentPropertyType::U32.is_integer());
        assert!(DocumentPropertyType::U64.is_integer());
    }

    #[test]
    fn test_is_integer_returns_false_for_non_integer_types() {
        assert!(!DocumentPropertyType::F64.is_integer());
        assert!(!DocumentPropertyType::Boolean.is_integer());
        assert!(!DocumentPropertyType::Date.is_integer());
        assert!(!DocumentPropertyType::Identifier.is_integer());
        assert!(!DocumentPropertyType::U128.is_integer());
        assert!(!DocumentPropertyType::I128.is_integer());
        assert!(!DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        })
        .is_integer());
    }

    // -----------------------------------------------------------------------
    // encode / decode roundtrip tests for tree keys
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_decode_u128_roundtrip() {
        let values: Vec<u128> = vec![0, 1, u128::MAX / 2, u128::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_u128(val);
            let decoded = DocumentPropertyType::decode_u128(&encoded).unwrap();
            assert_eq!(val, decoded, "u128 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_i128_roundtrip() {
        let values: Vec<i128> = vec![i128::MIN, -1, 0, 1, i128::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_i128(val);
            let decoded = DocumentPropertyType::decode_i128(&encoded).unwrap();
            assert_eq!(val, decoded, "i128 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_u64_roundtrip() {
        let values: Vec<u64> = vec![0, 1, 42, u64::MAX / 2, u64::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_u64(val);
            let decoded = DocumentPropertyType::decode_u64(&encoded).unwrap();
            assert_eq!(val, decoded, "u64 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_i64_roundtrip() {
        let values: Vec<i64> = vec![i64::MIN, -1, 0, 1, i64::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_i64(val);
            let decoded = DocumentPropertyType::decode_i64(&encoded).unwrap();
            assert_eq!(val, decoded, "i64 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_u32_roundtrip() {
        let values: Vec<u32> = vec![0, 1, u32::MAX / 2, u32::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_u32(val);
            let decoded = DocumentPropertyType::decode_u32(&encoded).unwrap();
            assert_eq!(val, decoded, "u32 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_i32_roundtrip() {
        let values: Vec<i32> = vec![i32::MIN, -1, 0, 1, i32::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_i32(val);
            let decoded = DocumentPropertyType::decode_i32(&encoded).unwrap();
            assert_eq!(val, decoded, "i32 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_u16_roundtrip() {
        let values: Vec<u16> = vec![0, 1, u16::MAX / 2, u16::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_u16(val);
            let decoded = DocumentPropertyType::decode_u16(&encoded).unwrap();
            assert_eq!(val, decoded, "u16 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_i16_roundtrip() {
        let values: Vec<i16> = vec![i16::MIN, -1, 0, 1, i16::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_i16(val);
            let decoded = DocumentPropertyType::decode_i16(&encoded).unwrap();
            assert_eq!(val, decoded, "i16 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_u8_roundtrip() {
        let values: Vec<u8> = vec![0, 1, 127, 255];
        for val in values {
            let encoded = DocumentPropertyType::encode_u8(val);
            let decoded = DocumentPropertyType::decode_u8(&encoded).unwrap();
            assert_eq!(val, decoded, "u8 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_i8_roundtrip() {
        let values: Vec<i8> = vec![i8::MIN, -1, 0, 1, i8::MAX];
        for val in values {
            let encoded = DocumentPropertyType::encode_i8(val);
            let decoded = DocumentPropertyType::decode_i8(&encoded).unwrap();
            assert_eq!(val, decoded, "i8 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_encode_decode_float_roundtrip() {
        let values: Vec<f64> = vec![-1000.5, -1.0, 0.0, 1.0, 42.42, 1000.5];
        for val in values {
            let encoded = DocumentPropertyType::encode_float(val);
            let decoded = DocumentPropertyType::decode_float(&encoded).unwrap();
            assert!(
                (val - decoded).abs() < f64::EPSILON,
                "float roundtrip failed for {}",
                val
            );
        }
    }

    #[test]
    fn test_encode_decode_date_timestamp_roundtrip() {
        let timestamps: Vec<u64> = vec![0, 1648910575000, u64::MAX];
        for ts in timestamps {
            let encoded = DocumentPropertyType::encode_date_timestamp(ts);
            let decoded = DocumentPropertyType::decode_date_timestamp(&encoded).unwrap();
            assert_eq!(ts, decoded, "date timestamp roundtrip failed for {}", ts);
        }
    }

    // -----------------------------------------------------------------------
    // encode sort order preservation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_u64_preserves_sort_order_in_lower_half() {
        // The encoding flips the sign bit, so sort order is preserved for
        // values in the lower half of the u64 range (0..2^63-1).
        let values: Vec<u64> = vec![0, 1, 100, 1000, i64::MAX as u64];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_u64(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for u64");
        }
    }

    #[test]
    fn test_encode_i64_preserves_sort_order() {
        let values: Vec<i64> = vec![i64::MIN, -100, -1, 0, 1, 100, i64::MAX];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_i64(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for i64");
        }
    }

    #[test]
    fn test_encode_float_preserves_sort_order() {
        let values: Vec<f64> = vec![-1000.0, -1.0, -0.5, 0.0, 0.5, 1.0, 1000.0];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_float(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for float");
        }
    }

    // -----------------------------------------------------------------------
    // encode_value_for_tree_keys() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_for_tree_keys_null_returns_empty() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_for_tree_keys(&Value::Null).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_encode_value_for_tree_keys_string_empty() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let result = prop
            .encode_value_for_tree_keys(&Value::Text("".to_string()))
            .unwrap();
        assert_eq!(result, vec![0]); // empty string marker
    }

    #[test]
    fn test_encode_value_for_tree_keys_string_nonempty() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let result = prop
            .encode_value_for_tree_keys(&Value::Text("hello".to_string()))
            .unwrap();
        assert_eq!(result, b"hello".to_vec());
    }

    #[test]
    fn test_encode_value_for_tree_keys_boolean() {
        let prop = DocumentPropertyType::Boolean;
        let true_enc = prop.encode_value_for_tree_keys(&Value::Bool(true)).unwrap();
        assert_eq!(true_enc, vec![1]);
        let false_enc = prop
            .encode_value_for_tree_keys(&Value::Bool(false))
            .unwrap();
        assert_eq!(false_enc, vec![0]);
    }

    #[test]
    fn test_encode_value_for_tree_keys_byte_array() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let bytes = vec![1u8, 2, 3, 4];
        let result = prop
            .encode_value_for_tree_keys(&Value::Bytes(bytes.clone()))
            .unwrap();
        assert_eq!(result, bytes);
    }

    #[test]
    fn test_encode_value_for_tree_keys_object_returns_error() {
        let prop = DocumentPropertyType::Object(IndexMap::new());
        let result = prop.encode_value_for_tree_keys(&Value::Map(vec![]));
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_for_tree_keys_array_returns_error() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let result = prop.encode_value_for_tree_keys(&Value::Array(vec![]));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // decode_value_for_tree_keys() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_decode_value_for_tree_keys_empty_returns_null() {
        let prop = DocumentPropertyType::U64;
        let result = prop.decode_value_for_tree_keys(&[]).unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn test_decode_value_for_tree_keys_string_empty_marker() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let result = prop.decode_value_for_tree_keys(&[0]).unwrap();
        assert_eq!(result, Value::Text("".to_string()));
    }

    #[test]
    fn test_decode_value_for_tree_keys_string() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let result = prop.decode_value_for_tree_keys(b"hello").unwrap();
        assert_eq!(result, Value::Text("hello".to_string()));
    }

    #[test]
    fn test_decode_value_for_tree_keys_boolean_true() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.decode_value_for_tree_keys(&[1]).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_decode_value_for_tree_keys_boolean_false() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.decode_value_for_tree_keys(&[0]).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_decode_value_for_tree_keys_boolean_invalid() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.decode_value_for_tree_keys(&[5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_value_for_tree_keys_byte_array() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let bytes = vec![10, 20, 30];
        let result = prop.decode_value_for_tree_keys(&bytes).unwrap();
        assert_eq!(result, Value::Bytes(bytes));
    }

    #[test]
    fn test_decode_value_for_tree_keys_object_returns_error() {
        let prop = DocumentPropertyType::Object(IndexMap::new());
        let result = prop.decode_value_for_tree_keys(&[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_value_for_tree_keys_array_returns_error() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let result = prop.decode_value_for_tree_keys(&[1, 2, 3]);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // encode_value_for_tree_keys() / decode_value_for_tree_keys() roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn test_tree_keys_roundtrip_all_integer_types() {
        // U64
        let prop = DocumentPropertyType::U64;
        let val = Value::U64(12345);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // I64
        let prop = DocumentPropertyType::I64;
        let val = Value::I64(-42);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // U32
        let prop = DocumentPropertyType::U32;
        let val = Value::U32(999);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // I32
        let prop = DocumentPropertyType::I32;
        let val = Value::I32(-100);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // U16
        let prop = DocumentPropertyType::U16;
        let val = Value::U16(500);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // I16
        let prop = DocumentPropertyType::I16;
        let val = Value::I16(-200);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // U8
        let prop = DocumentPropertyType::U8;
        let val = Value::U8(42);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // I8
        let prop = DocumentPropertyType::I8;
        let val = Value::I8(-5);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // U128
        let prop = DocumentPropertyType::U128;
        let val = Value::U128(99999);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);

        // I128
        let prop = DocumentPropertyType::I128;
        let val = Value::I128(-99999);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);
    }

    #[test]
    fn test_tree_keys_roundtrip_float() {
        let prop = DocumentPropertyType::F64;
        let val = Value::Float(3.14);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        if let Value::Float(f) = dec {
            assert!((f - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("expected float value");
        }
    }

    #[test]
    fn test_tree_keys_roundtrip_date() {
        let prop = DocumentPropertyType::Date;
        let val = Value::U64(1648910575000);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, val);
    }

    #[test]
    fn test_tree_keys_roundtrip_identifier() {
        let prop = DocumentPropertyType::Identifier;
        let id_bytes: [u8; 32] = [42u8; 32];
        let val = Value::Identifier(id_bytes);
        let enc = prop.encode_value_for_tree_keys(&val).unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        // Identifier decodes via Identifier::from_bytes, which gives Identifier variant
        if let Value::Identifier(decoded_id) = dec {
            assert_eq!(decoded_id, id_bytes);
        } else {
            panic!("expected identifier value");
        }
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_with_size_null_returns_empty() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_with_size(Value::Null, true).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_encode_value_with_size_string() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let result = prop
            .encode_value_with_size(Value::Text("hi".to_string()), true)
            .unwrap();
        // varint(2) + b"hi" = [2, 104, 105]
        assert_eq!(result.len(), 3);
        assert_eq!(&result[1..], b"hi");
    }

    #[test]
    fn test_encode_value_with_size_boolean() {
        let prop = DocumentPropertyType::Boolean;
        let true_result = prop
            .encode_value_with_size(Value::Bool(true), true)
            .unwrap();
        assert_eq!(true_result, vec![1]);
        let false_result = prop
            .encode_value_with_size(Value::Bool(false), true)
            .unwrap();
        assert_eq!(false_result, vec![2]);
    }

    #[test]
    fn test_encode_value_with_size_u64_required() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_with_size(Value::U64(42), true).unwrap();
        assert_eq!(result.len(), 8); // 8 bytes for u64
        assert_eq!(result, 42u64.to_be_bytes().to_vec());
    }

    #[test]
    fn test_encode_value_with_size_u64_not_required() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_with_size(Value::U64(42), false).unwrap();
        assert_eq!(result.len(), 9); // 1 byte marker + 8 bytes
        assert_eq!(result[0], 255u8); // marker byte
        assert_eq!(&result[1..], 42u64.to_be_bytes().as_slice());
    }

    #[test]
    fn test_encode_value_with_size_i64_required() {
        let prop = DocumentPropertyType::I64;
        let result = prop.encode_value_with_size(Value::I64(-42), true).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_with_size_f64_required() {
        let prop = DocumentPropertyType::F64;
        let result = prop
            .encode_value_with_size(Value::Float(3.14), true)
            .unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result, 3.14f64.to_be_bytes().to_vec());
    }

    #[test]
    fn test_encode_value_with_size_f64_not_required() {
        let prop = DocumentPropertyType::F64;
        let result = prop
            .encode_value_with_size(Value::Float(3.14), false)
            .unwrap();
        assert_eq!(result.len(), 9);
        assert_eq!(result[0], 255u8);
    }

    #[test]
    fn test_encode_value_with_size_date_required() {
        let prop = DocumentPropertyType::Date;
        let result = prop
            .encode_value_with_size(Value::Float(1648910575.0), true)
            .unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_with_size_byte_array() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let bytes = vec![1u8, 2, 3];
        let result = prop
            .encode_value_with_size(Value::Bytes(bytes), true)
            .unwrap();
        // varint(3) + [1,2,3]
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_with_size_identifier() {
        let prop = DocumentPropertyType::Identifier;
        let id_bytes = [1u8; 32];
        let result = prop
            .encode_value_with_size(Value::Identifier(id_bytes), true)
            .unwrap();
        // varint(32) + 32 bytes
        assert_eq!(result.len(), 33);
    }

    #[test]
    fn test_encode_value_with_size_u128_required() {
        let prop = DocumentPropertyType::U128;
        let result = prop
            .encode_value_with_size(Value::U128(1000), true)
            .unwrap();
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_encode_value_with_size_u128_not_required() {
        let prop = DocumentPropertyType::U128;
        let result = prop
            .encode_value_with_size(Value::U128(1000), false)
            .unwrap();
        assert_eq!(result.len(), 17); // 1 marker + 16
        assert_eq!(result[0], 255u8);
    }

    #[test]
    fn test_encode_value_with_size_i128_required() {
        let prop = DocumentPropertyType::I128;
        let result = prop
            .encode_value_with_size(Value::I128(-500), true)
            .unwrap();
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_encode_value_with_size_u32_required() {
        let prop = DocumentPropertyType::U32;
        let result = prop.encode_value_with_size(Value::U32(100), true).unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(result, 100u32.to_be_bytes().to_vec());
    }

    #[test]
    fn test_encode_value_with_size_i32_required() {
        let prop = DocumentPropertyType::I32;
        let result = prop.encode_value_with_size(Value::I32(-50), true).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_with_size_u16_required() {
        let prop = DocumentPropertyType::U16;
        let result = prop.encode_value_with_size(Value::U16(300), true).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result, 300u16.to_be_bytes().to_vec());
    }

    #[test]
    fn test_encode_value_with_size_i16_required() {
        let prop = DocumentPropertyType::I16;
        let result = prop.encode_value_with_size(Value::I16(-100), true).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_encode_value_with_size_u8_required() {
        let prop = DocumentPropertyType::U8;
        let result = prop.encode_value_with_size(Value::U8(42), true).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_encode_value_with_size_i8_required() {
        let prop = DocumentPropertyType::I8;
        let result = prop.encode_value_with_size(Value::I8(-10), true).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_encode_value_with_size_variable_type_array_returns_error() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let result = prop.encode_value_with_size(Value::Array(vec![]), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_string_type_mismatch() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let result = prop.encode_value_with_size(Value::U64(42), true);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // encode_value_ref_with_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_ref_with_size_null_returns_empty() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_ref_with_size(&Value::Null, true).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_encode_value_ref_with_size_string() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let val = Value::Text("test".to_string());
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 5); // varint(4) + "test"
    }

    #[test]
    fn test_encode_value_ref_with_size_date_required() {
        let prop = DocumentPropertyType::Date;
        let val = Value::Float(1648910575.0);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_date_not_required() {
        let prop = DocumentPropertyType::Date;
        let val = Value::Float(1648910575.0);
        let result = prop.encode_value_ref_with_size(&val, false).unwrap();
        assert_eq!(result.len(), 9); // marker + 8 bytes
        assert_eq!(result[0], 255u8);
    }

    #[test]
    fn test_encode_value_ref_with_size_u128() {
        let prop = DocumentPropertyType::U128;
        let val = Value::U128(42);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_encode_value_ref_with_size_i128() {
        let prop = DocumentPropertyType::I128;
        let val = Value::I128(-42);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_encode_value_ref_with_size_u64() {
        let prop = DocumentPropertyType::U64;
        let val = Value::U64(100);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_i64() {
        let prop = DocumentPropertyType::I64;
        let val = Value::I64(-100);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_u32() {
        let prop = DocumentPropertyType::U32;
        let val = Value::U32(50);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_ref_with_size_i32() {
        let prop = DocumentPropertyType::I32;
        let val = Value::I32(-50);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_ref_with_size_u16() {
        let prop = DocumentPropertyType::U16;
        let val = Value::U16(300);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_encode_value_ref_with_size_i16() {
        let prop = DocumentPropertyType::I16;
        let val = Value::I16(-300);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_encode_value_ref_with_size_u8() {
        let prop = DocumentPropertyType::U8;
        let val = Value::U8(255);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_encode_value_ref_with_size_i8() {
        let prop = DocumentPropertyType::I8;
        let val = Value::I8(-128);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_encode_value_ref_with_size_f64() {
        let prop = DocumentPropertyType::F64;
        let val = Value::Float(2.718);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_boolean() {
        let prop = DocumentPropertyType::Boolean;
        let result_true = prop
            .encode_value_ref_with_size(&Value::Bool(true), true)
            .unwrap();
        assert_eq!(result_true, vec![1]);
        let result_false = prop
            .encode_value_ref_with_size(&Value::Bool(false), true)
            .unwrap();
        assert_eq!(result_false, vec![0]);
    }

    #[test]
    fn test_encode_value_ref_with_size_byte_array_fixed_size() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(4),
            max_size: Some(4),
        });
        let val = Value::Bytes(vec![1, 2, 3, 4]);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        // fixed size: no varint prefix
        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_encode_value_ref_with_size_byte_array_variable_size() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(1),
            max_size: Some(10),
        });
        let val = Value::Bytes(vec![10, 20, 30]);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        // varint(3) + [10,20,30]
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_ref_with_size_identifier() {
        let prop = DocumentPropertyType::Identifier;
        let id_bytes = [5u8; 32];
        let val = Value::Identifier(id_bytes);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_encode_value_ref_with_size_variable_type_array_returns_error() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let val = Value::Array(vec![]);
        let result = prop.encode_value_ref_with_size(&val, true);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // value_from_string() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_value_from_string_string_type() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let result = prop.value_from_string("hello").unwrap();
        assert_eq!(result, Value::Text("hello".to_string()));
    }

    #[test]
    fn test_value_from_string_string_too_small() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(10),
            max_length: None,
        });
        let result = prop.value_from_string("hi");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_from_string_string_too_big() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(3),
        });
        let result = prop.value_from_string("hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_from_string_u128() {
        let prop = DocumentPropertyType::U128;
        let result = prop
            .value_from_string("340282366920938463463374607431768211455")
            .unwrap();
        assert_eq!(result, Value::U128(u128::MAX));
    }

    #[test]
    fn test_value_from_string_u128_invalid() {
        let prop = DocumentPropertyType::U128;
        assert!(prop.value_from_string("not_a_number").is_err());
    }

    #[test]
    fn test_value_from_string_i128() {
        let prop = DocumentPropertyType::I128;
        let result = prop.value_from_string("-1").unwrap();
        assert_eq!(result, Value::I128(-1));
    }

    #[test]
    fn test_value_from_string_i128_invalid() {
        let prop = DocumentPropertyType::I128;
        assert!(prop.value_from_string("abc").is_err());
    }

    #[test]
    fn test_value_from_string_u64() {
        let prop = DocumentPropertyType::U64;
        let result = prop.value_from_string("12345").unwrap();
        assert_eq!(result, Value::U64(12345));
    }

    #[test]
    fn test_value_from_string_u64_invalid() {
        let prop = DocumentPropertyType::U64;
        assert!(prop.value_from_string("-1").is_err());
    }

    #[test]
    fn test_value_from_string_i64() {
        let prop = DocumentPropertyType::I64;
        let result = prop.value_from_string("-42").unwrap();
        assert_eq!(result, Value::I64(-42));
    }

    #[test]
    fn test_value_from_string_u32() {
        let prop = DocumentPropertyType::U32;
        let result = prop.value_from_string("1000").unwrap();
        assert_eq!(result, Value::U32(1000));
    }

    #[test]
    fn test_value_from_string_i32() {
        let prop = DocumentPropertyType::I32;
        let result = prop.value_from_string("-1000").unwrap();
        assert_eq!(result, Value::I32(-1000));
    }

    #[test]
    fn test_value_from_string_u16() {
        let prop = DocumentPropertyType::U16;
        let result = prop.value_from_string("65535").unwrap();
        assert_eq!(result, Value::U16(65535));
    }

    #[test]
    fn test_value_from_string_i16() {
        let prop = DocumentPropertyType::I16;
        let result = prop.value_from_string("-32768").unwrap();
        assert_eq!(result, Value::I16(-32768));
    }

    #[test]
    fn test_value_from_string_u8() {
        let prop = DocumentPropertyType::U8;
        let result = prop.value_from_string("255").unwrap();
        assert_eq!(result, Value::U8(255));
    }

    #[test]
    fn test_value_from_string_u8_invalid() {
        let prop = DocumentPropertyType::U8;
        assert!(prop.value_from_string("256").is_err());
    }

    #[test]
    fn test_value_from_string_i8() {
        let prop = DocumentPropertyType::I8;
        let result = prop.value_from_string("-128").unwrap();
        assert_eq!(result, Value::I8(-128));
    }

    #[test]
    fn test_value_from_string_f64() {
        let prop = DocumentPropertyType::F64;
        let result = prop.value_from_string("3.14").unwrap();
        if let Value::Float(f) = result {
            assert!((f - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("expected float");
        }
    }

    #[test]
    fn test_value_from_string_date() {
        let prop = DocumentPropertyType::Date;
        let result = prop.value_from_string("1648910575.0").unwrap();
        assert!(matches!(result, Value::Float(_)));
    }

    #[test]
    fn test_value_from_string_byte_array() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let result = prop.value_from_string("deadbeef").unwrap();
        assert_eq!(result, Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    }

    #[test]
    fn test_value_from_string_byte_array_too_small() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(10),
            max_size: None,
        });
        let result = prop.value_from_string("aabb");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_from_string_byte_array_too_big() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: Some(2),
        });
        let result = prop.value_from_string("aabbccddee");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_from_string_byte_array_invalid_hex() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let result = prop.value_from_string("not_hex");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_from_string_boolean_true() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.value_from_string("true").unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_value_from_string_boolean_false() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.value_from_string("false").unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_value_from_string_boolean_case_insensitive() {
        let prop = DocumentPropertyType::Boolean;
        assert_eq!(prop.value_from_string("TRUE").unwrap(), Value::Bool(true));
        assert_eq!(prop.value_from_string("False").unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_value_from_string_boolean_invalid() {
        let prop = DocumentPropertyType::Boolean;
        assert!(prop.value_from_string("yes").is_err());
    }

    #[test]
    fn test_value_from_string_object_returns_error() {
        let prop = DocumentPropertyType::Object(IndexMap::new());
        assert!(prop.value_from_string("{}").is_err());
    }

    #[test]
    fn test_value_from_string_array_returns_error() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        assert!(prop.value_from_string("[]").is_err());
    }

    // -----------------------------------------------------------------------
    // read_optionally_from() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_read_optionally_from_optional_marker_none() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U64;
        // byte 0 means "not present"
        let data: &[u8] = &[0];
        let mut reader = BufReader::new(data);
        let (value, finished) = prop.read_optionally_from(&mut reader, false).unwrap();
        assert!(value.is_none());
        assert!(!finished);
    }

    #[test]
    fn test_read_optionally_from_optional_marker_eof() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U64;
        let data: &[u8] = &[];
        let mut reader = BufReader::new(data);
        let (value, finished) = prop.read_optionally_from(&mut reader, false).unwrap();
        assert!(value.is_none());
        assert!(finished); // EOF = finished
    }

    #[test]
    fn test_read_optionally_from_u64_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U64;
        let data = 42u64.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, finished) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::U64(42)));
        assert!(!finished);
    }

    #[test]
    fn test_read_optionally_from_boolean_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::Boolean;
        // 0 = false
        let data: &[u8] = &[0];
        let mut reader = BufReader::new(data);
        let (value, finished) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bool(false)));
        assert!(!finished);

        // non-zero = true
        let data: &[u8] = &[1];
        let mut reader = BufReader::new(data);
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bool(true)));
    }

    #[test]
    fn test_read_optionally_from_string_required() {
        use integer_encoding::VarInt;
        use std::io::BufReader;
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let text = b"hello";
        let mut data = text.len().encode_var_vec();
        data.extend_from_slice(text);
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Text("hello".to_string())));
    }

    #[test]
    fn test_read_optionally_from_identifier_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::Identifier;
        let id_bytes = [7u8; 32];
        let mut reader = BufReader::new(id_bytes.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Identifier(id_bytes)));
    }

    #[test]
    fn test_read_optionally_from_f64_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::F64;
        let data = 3.14f64.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        if let Some(Value::Float(f)) = value {
            assert!((f - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("expected float");
        }
    }

    #[test]
    fn test_read_optionally_from_i128_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::I128;
        let data = (-999i128).to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::I128(-999)));
    }

    #[test]
    fn test_read_optionally_from_u128_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U128;
        let data = 999u128.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::U128(999)));
    }

    #[test]
    fn test_read_optionally_from_i64_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::I64;
        let data = (-42i64).to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::I64(-42)));
    }

    #[test]
    fn test_read_optionally_from_u32_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U32;
        let data = 100u32.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::U32(100)));
    }

    #[test]
    fn test_read_optionally_from_i32_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::I32;
        let data = (-100i32).to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::I32(-100)));
    }

    #[test]
    fn test_read_optionally_from_u16_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U16;
        let data = 300u16.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::U16(300)));
    }

    #[test]
    fn test_read_optionally_from_i16_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::I16;
        let data = (-300i16).to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::I16(-300)));
    }

    #[test]
    fn test_read_optionally_from_u8_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::U8;
        let data: &[u8] = &[42];
        let mut reader = BufReader::new(data);
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::U8(42)));
    }

    #[test]
    fn test_read_optionally_from_i8_required() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::I8;
        let data: &[u8] = &[(-10i8) as u8];
        let mut reader = BufReader::new(data);
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::I8(-10)));
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_size() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(4),
            max_size: Some(4),
        });
        let data: &[u8] = &[1, 2, 3, 4];
        let mut reader = BufReader::new(data);
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes(vec![1, 2, 3, 4])));
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_32() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(32),
            max_size: Some(32),
        });
        let data = [99u8; 32];
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes32(data)));
    }

    #[test]
    fn test_read_optionally_from_byte_array_variable() {
        use integer_encoding::VarInt;
        use std::io::BufReader;
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(1),
            max_size: Some(10),
        });
        let bytes = vec![10, 20, 30];
        let mut data = bytes.len().encode_var_vec();
        data.extend_from_slice(&bytes);
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes(vec![10, 20, 30])));
    }

    #[test]
    fn test_read_optionally_from_array_returns_error() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let data: &[u8] = &[1, 2, 3];
        let mut reader = BufReader::new(data);
        let result = prop.read_optionally_from(&mut reader, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_optionally_from_variable_type_array_returns_error() {
        use std::io::BufReader;
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let data: &[u8] = &[1, 2, 3];
        let mut reader = BufReader::new(data);
        let result = prop.read_optionally_from(&mut reader, true);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // sanitize_value_mut() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanitize_value_mut_u8_from_u16() {
        let prop = DocumentPropertyType::U8;
        let mut val = Value::U16(200);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U8(200));
    }

    #[test]
    fn test_sanitize_value_mut_u8_from_u32() {
        let prop = DocumentPropertyType::U8;
        let mut val = Value::U32(100);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U8(100));
    }

    #[test]
    fn test_sanitize_value_mut_u8_from_u64() {
        let prop = DocumentPropertyType::U8;
        let mut val = Value::U64(50);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U8(50));
    }

    #[test]
    fn test_sanitize_value_mut_u8_from_u128() {
        let prop = DocumentPropertyType::U8;
        let mut val = Value::U128(10);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U8(10));
    }

    #[test]
    fn test_sanitize_value_mut_u16_from_u8() {
        let prop = DocumentPropertyType::U16;
        let mut val = Value::U8(100);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U16(100));
    }

    #[test]
    fn test_sanitize_value_mut_u32_from_u8() {
        let prop = DocumentPropertyType::U32;
        let mut val = Value::U8(100);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U32(100));
    }

    #[test]
    fn test_sanitize_value_mut_u64_from_u32() {
        let prop = DocumentPropertyType::U64;
        let mut val = Value::U32(1000);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U64(1000));
    }

    #[test]
    fn test_sanitize_value_mut_u128_from_u64() {
        let prop = DocumentPropertyType::U128;
        let mut val = Value::U64(1000);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U128(1000));
    }

    #[test]
    fn test_sanitize_value_mut_i8_from_i16() {
        let prop = DocumentPropertyType::I8;
        let mut val = Value::I16(-50);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I8(-50));
    }

    #[test]
    fn test_sanitize_value_mut_i16_from_i8() {
        let prop = DocumentPropertyType::I16;
        let mut val = Value::I8(-10);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I16(-10));
    }

    #[test]
    fn test_sanitize_value_mut_i32_from_i16() {
        let prop = DocumentPropertyType::I32;
        let mut val = Value::I16(-100);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I32(-100));
    }

    #[test]
    fn test_sanitize_value_mut_i64_from_i32() {
        let prop = DocumentPropertyType::I64;
        let mut val = Value::I32(-1000);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I64(-1000));
    }

    #[test]
    fn test_sanitize_value_mut_i128_from_i64() {
        let prop = DocumentPropertyType::I128;
        let mut val = Value::I64(-50000);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I128(-50000));
    }

    #[test]
    fn test_sanitize_value_mut_date_from_i64() {
        let prop = DocumentPropertyType::Date;
        let mut val = Value::I64(1648910575000);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U64(1648910575000));
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_from_hex_string() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let mut val = Value::Text("deadbeef".to_string());
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    }

    #[test]
    fn test_sanitize_value_mut_leaves_unrelated_type_unchanged() {
        let prop = DocumentPropertyType::U64;
        let mut val = Value::Text("hello".to_string());
        prop.sanitize_value_mut(&mut val);
        // Should not change since String doesn't match U64 sanitization
        assert_eq!(val, Value::Text("hello".to_string()));
    }

    // -----------------------------------------------------------------------
    // try_from_value_map() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_try_from_value_map_string_type() {
        let type_val = Value::Text("string".to_string());
        let min_val = Value::U64(5);
        let max_val = Value::U64(100);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("minLength".to_string(), &min_val);
        map.insert("maxLength".to_string(), &max_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(
            result,
            DocumentPropertyType::String(StringPropertySizes {
                min_length: Some(5),
                max_length: Some(100),
            })
        );
    }

    #[test]
    fn test_try_from_value_map_boolean_type() {
        let type_val = Value::Text("boolean".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::Boolean);
    }

    #[test]
    fn test_try_from_value_map_number_type() {
        let type_val = Value::Text("number".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::F64);
    }

    #[test]
    fn test_try_from_value_map_integer_with_min_max() {
        let type_val = Value::Text("integer".to_string());
        let min_val = Value::I64(0);
        let max_val = Value::I64(255);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("minimum".to_string(), &min_val);
        map.insert("maximum".to_string(), &max_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::U8);
    }

    #[test]
    fn test_try_from_value_map_integer_no_sized() {
        let type_val = Value::Text("integer".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: false,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::I64);
    }

    #[test]
    fn test_try_from_value_map_unsupported_type() {
        let type_val = Value::Text("map".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_value_map_array_byte_array_identifier() {
        let type_val = Value::Text("array".to_string());
        let byte_array_val = Value::Bool(true);
        let media_type_val = Value::Text("application/x.dash.dpp.identifier".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("byteArray".to_string(), &byte_array_val);
        map.insert("contentMediaType".to_string(), &media_type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::Identifier);
    }

    #[test]
    fn test_try_from_value_map_array_byte_array_plain() {
        let type_val = Value::Text("array".to_string());
        let byte_array_val = Value::Bool(true);
        let min_items_val = Value::U64(10);
        let max_items_val = Value::U64(50);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("byteArray".to_string(), &byte_array_val);
        map.insert("minItems".to_string(), &min_items_val);
        map.insert("maxItems".to_string(), &max_items_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(
            result,
            DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
                min_size: Some(10),
                max_size: Some(50),
            })
        );
    }

    #[test]
    fn test_try_from_value_map_array_not_byte_array_errors() {
        let type_val = Value::Text("array".to_string());
        let byte_array_val = Value::Bool(false);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("byteArray".to_string(), &byte_array_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_value_map_array_no_byte_array_flag_errors() {
        let type_val = Value::Text("array".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // find_integer_type helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_unsigned_integer_type_for_max_value() {
        assert_eq!(
            find_unsigned_integer_type_for_max_value(100),
            DocumentPropertyType::U8
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(255),
            DocumentPropertyType::U8
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(256),
            DocumentPropertyType::U16
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(65535),
            DocumentPropertyType::U16
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(65536),
            DocumentPropertyType::U32
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(u32::MAX as i64),
            DocumentPropertyType::U32
        );
        assert_eq!(
            find_unsigned_integer_type_for_max_value(u32::MAX as i64 + 1),
            DocumentPropertyType::U64
        );
    }

    #[test]
    fn test_find_integer_type_for_min_and_max_values() {
        // positive range -> unsigned
        assert_eq!(
            find_integer_type_for_min_and_max_values(0, 255),
            DocumentPropertyType::U8
        );
        // signed ranges
        assert_eq!(
            find_integer_type_for_min_and_max_values(-128, 127),
            DocumentPropertyType::I8
        );
        assert_eq!(
            find_integer_type_for_min_and_max_values(-32768, 32767),
            DocumentPropertyType::I16
        );
        assert_eq!(
            find_integer_type_for_min_and_max_values(i32::MIN as i64, i32::MAX as i64),
            DocumentPropertyType::I32
        );
        assert_eq!(
            find_integer_type_for_min_and_max_values(i64::MIN, i64::MAX),
            DocumentPropertyType::I64
        );
    }

    // -----------------------------------------------------------------------
    // DocumentPropertyTypeParsingOptions tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_parsing_options_default() {
        let opts = DocumentPropertyTypeParsingOptions::default();
        assert!(opts.sized_integer_types);
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() round-trip with read_optionally_from()
    // -----------------------------------------------------------------------

    /// Helper: encode a value with `encode_value_with_size`, then decode it
    /// with `read_optionally_from` and return the decoded value.
    fn roundtrip_encode_read(prop: &DocumentPropertyType, value: Value, required: bool) -> Value {
        let encoded = prop
            .encode_value_with_size(value, required)
            .expect("encode should succeed");
        let mut reader = BufReader::new(encoded.as_slice());
        let (decoded, _finished) = prop
            .read_optionally_from(&mut reader, required)
            .expect("read should succeed");
        decoded.expect("decoded value should be Some")
    }

    #[test]
    fn test_roundtrip_u8_required() {
        let prop = DocumentPropertyType::U8;
        for val in [0u8, 1, 127, 255] {
            let decoded = roundtrip_encode_read(&prop, Value::U8(val), true);
            assert_eq!(decoded, Value::U8(val), "u8 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_u16_required() {
        let prop = DocumentPropertyType::U16;
        for val in [0u16, 1, 300, u16::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::U16(val), true);
            assert_eq!(decoded, Value::U16(val), "u16 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_u32_required() {
        let prop = DocumentPropertyType::U32;
        for val in [0u32, 1, 100_000, u32::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::U32(val), true);
            assert_eq!(decoded, Value::U32(val), "u32 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_u64_required() {
        let prop = DocumentPropertyType::U64;
        for val in [0u64, 1, 1_000_000, u64::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::U64(val), true);
            assert_eq!(decoded, Value::U64(val), "u64 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_u128_required() {
        let prop = DocumentPropertyType::U128;
        for val in [0u128, 1, u128::MAX / 2, u128::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::U128(val), true);
            assert_eq!(
                decoded,
                Value::U128(val),
                "u128 roundtrip failed for {}",
                val
            );
        }
    }

    #[test]
    fn test_roundtrip_i8_required() {
        let prop = DocumentPropertyType::I8;
        for val in [i8::MIN, -1, 0, 1, i8::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::I8(val), true);
            assert_eq!(decoded, Value::I8(val), "i8 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_i16_required() {
        let prop = DocumentPropertyType::I16;
        for val in [i16::MIN, -1, 0, 1, i16::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::I16(val), true);
            assert_eq!(decoded, Value::I16(val), "i16 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_i32_required() {
        let prop = DocumentPropertyType::I32;
        for val in [i32::MIN, -1, 0, 1, i32::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::I32(val), true);
            assert_eq!(decoded, Value::I32(val), "i32 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_i64_required() {
        let prop = DocumentPropertyType::I64;
        for val in [i64::MIN, -1, 0, 1, i64::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::I64(val), true);
            assert_eq!(decoded, Value::I64(val), "i64 roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_roundtrip_i128_required() {
        let prop = DocumentPropertyType::I128;
        for val in [i128::MIN, -1, 0, 1, i128::MAX] {
            let decoded = roundtrip_encode_read(&prop, Value::I128(val), true);
            assert_eq!(
                decoded,
                Value::I128(val),
                "i128 roundtrip failed for {}",
                val
            );
        }
    }

    #[test]
    fn test_roundtrip_f64_required() {
        let prop = DocumentPropertyType::F64;
        for val in [-1000.5f64, -1.0, 0.0, 1.0, 3.14, 1000.5] {
            let decoded = roundtrip_encode_read(&prop, Value::Float(val), true);
            if let Value::Float(f) = decoded {
                assert!(
                    (f - val).abs() < f64::EPSILON,
                    "f64 roundtrip failed for {}",
                    val
                );
            } else {
                panic!("expected float, got {:?}", decoded);
            }
        }
    }

    #[test]
    fn test_roundtrip_date_required() {
        let prop = DocumentPropertyType::Date;
        let val = 1648910575.0f64;
        let decoded = roundtrip_encode_read(&prop, Value::Float(val), true);
        if let Value::Float(f) = decoded {
            assert!((f - val).abs() < f64::EPSILON);
        } else {
            panic!("expected float for date");
        }
    }

    #[test]
    fn test_roundtrip_boolean_true_required() {
        let prop = DocumentPropertyType::Boolean;
        // encode_value_with_size encodes true as [1], read_optionally_from
        // interprets non-zero as true
        let decoded = roundtrip_encode_read(&prop, Value::Bool(true), true);
        assert_eq!(decoded, Value::Bool(true));
    }

    #[test]
    fn test_roundtrip_boolean_false_required() {
        let prop = DocumentPropertyType::Boolean;
        // encode_value_with_size encodes false as [2], read_optionally_from
        // interprets non-zero as true -- this is the actual behavior
        let decoded = roundtrip_encode_read(&prop, Value::Bool(false), true);
        // Note: encode uses 2 for false, but read interprets any non-zero as true.
        // This documents the actual (asymmetric) behavior of the production code.
        assert_eq!(decoded, Value::Bool(true));
    }

    #[test]
    fn test_roundtrip_string_empty() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let decoded = roundtrip_encode_read(&prop, Value::Text("".to_string()), true);
        assert_eq!(decoded, Value::Text("".to_string()));
    }

    #[test]
    fn test_roundtrip_string_short() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(100),
        });
        let decoded = roundtrip_encode_read(&prop, Value::Text("hello world".to_string()), true);
        assert_eq!(decoded, Value::Text("hello world".to_string()));
    }

    #[test]
    fn test_roundtrip_string_long() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: Some(1000),
        });
        let long_string = "a".repeat(500);
        let decoded = roundtrip_encode_read(&prop, Value::Text(long_string.clone()), true);
        assert_eq!(decoded, Value::Text(long_string));
    }

    #[test]
    fn test_roundtrip_byte_array_variable_size() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(1),
            max_size: Some(100),
        });
        let bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let decoded = roundtrip_encode_read(&prop, Value::Bytes(bytes.clone()), true);
        assert_eq!(decoded, Value::Bytes(bytes));
    }

    #[test]
    fn test_roundtrip_byte_array_empty_variable() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(0),
            max_size: Some(100),
        });
        let decoded = roundtrip_encode_read(&prop, Value::Bytes(vec![]), true);
        assert_eq!(decoded, Value::Bytes(vec![]));
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() optional (non-required) round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_roundtrip_u64_optional_present() {
        let prop = DocumentPropertyType::U64;
        let decoded = roundtrip_encode_read(&prop, Value::U64(42), false);
        assert_eq!(decoded, Value::U64(42));
    }

    #[test]
    fn test_roundtrip_i64_optional_present() {
        let prop = DocumentPropertyType::I64;
        let decoded = roundtrip_encode_read(&prop, Value::I64(-999), false);
        assert_eq!(decoded, Value::I64(-999));
    }

    #[test]
    fn test_roundtrip_u32_optional_present() {
        let prop = DocumentPropertyType::U32;
        let decoded = roundtrip_encode_read(&prop, Value::U32(12345), false);
        assert_eq!(decoded, Value::U32(12345));
    }

    #[test]
    fn test_roundtrip_i32_optional_present() {
        let prop = DocumentPropertyType::I32;
        let decoded = roundtrip_encode_read(&prop, Value::I32(-12345), false);
        assert_eq!(decoded, Value::I32(-12345));
    }

    #[test]
    fn test_roundtrip_u16_optional_present() {
        let prop = DocumentPropertyType::U16;
        let decoded = roundtrip_encode_read(&prop, Value::U16(500), false);
        assert_eq!(decoded, Value::U16(500));
    }

    #[test]
    fn test_roundtrip_i16_optional_present() {
        let prop = DocumentPropertyType::I16;
        let decoded = roundtrip_encode_read(&prop, Value::I16(-500), false);
        assert_eq!(decoded, Value::I16(-500));
    }

    #[test]
    fn test_roundtrip_u8_optional_present() {
        let prop = DocumentPropertyType::U8;
        let decoded = roundtrip_encode_read(&prop, Value::U8(200), false);
        assert_eq!(decoded, Value::U8(200));
    }

    #[test]
    fn test_roundtrip_i8_optional_present() {
        let prop = DocumentPropertyType::I8;
        let decoded = roundtrip_encode_read(&prop, Value::I8(-100), false);
        assert_eq!(decoded, Value::I8(-100));
    }

    #[test]
    fn test_roundtrip_u128_optional_present() {
        let prop = DocumentPropertyType::U128;
        let decoded = roundtrip_encode_read(&prop, Value::U128(99999), false);
        assert_eq!(decoded, Value::U128(99999));
    }

    #[test]
    fn test_roundtrip_i128_optional_present() {
        let prop = DocumentPropertyType::I128;
        let decoded = roundtrip_encode_read(&prop, Value::I128(-99999), false);
        assert_eq!(decoded, Value::I128(-99999));
    }

    #[test]
    fn test_roundtrip_f64_optional_present() {
        let prop = DocumentPropertyType::F64;
        let decoded = roundtrip_encode_read(&prop, Value::Float(2.718), false);
        if let Value::Float(f) = decoded {
            assert!((f - 2.718).abs() < f64::EPSILON);
        } else {
            panic!("expected float");
        }
    }

    #[test]
    fn test_roundtrip_date_optional_present() {
        let prop = DocumentPropertyType::Date;
        let val = 1648910575.0f64;
        let decoded = roundtrip_encode_read(&prop, Value::Float(val), false);
        if let Value::Float(f) = decoded {
            assert!((f - val).abs() < f64::EPSILON);
        } else {
            panic!("expected float for date");
        }
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() for Object with nested fields
    // -----------------------------------------------------------------------

    #[test]
    fn test_roundtrip_object_with_nested_fields() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "name".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::String(StringPropertySizes {
                    min_length: None,
                    max_length: Some(100),
                }),
                required: true,
                transient: false,
            },
        );
        inner_fields.insert(
            "age".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);

        let value = Value::Map(vec![
            (
                Value::Text("name".to_string()),
                Value::Text("Alice".to_string()),
            ),
            (Value::Text("age".to_string()), Value::U32(30)),
        ]);

        let encoded = prop
            .encode_value_with_size(value, true)
            .expect("encode object should succeed");

        // Decode it back
        let mut reader = BufReader::new(encoded.as_slice());
        let (decoded, _) = prop
            .read_optionally_from(&mut reader, true)
            .expect("read object should succeed");

        let decoded = decoded.expect("decoded should be Some");
        if let Value::Map(map) = decoded {
            assert_eq!(map.len(), 2);
            assert_eq!(
                map[0],
                (
                    Value::Text("name".to_string()),
                    Value::Text("Alice".to_string())
                )
            );
            assert_eq!(map[1], (Value::Text("age".to_string()), Value::U32(30)));
        } else {
            panic!("expected Map value, got {:?}", decoded);
        }
    }

    #[test]
    fn test_encode_value_with_size_object_missing_required_field() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "name".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::String(StringPropertySizes {
                    min_length: None,
                    max_length: Some(100),
                }),
                required: true,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);

        // Empty map -- missing required "name" field
        let value = Value::Map(vec![]);
        let result = prop.encode_value_with_size(value, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_object_with_optional_field_absent() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "required_field".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
            },
        );
        inner_fields.insert(
            "optional_field".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U64,
                required: false,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);

        // Only provide the required field
        let value = Value::Map(vec![(
            Value::Text("required_field".to_string()),
            Value::U32(42),
        )]);

        let encoded = prop
            .encode_value_with_size(value, true)
            .expect("encode should succeed");

        let mut reader = BufReader::new(encoded.as_slice());
        let (decoded, _) = prop
            .read_optionally_from(&mut reader, true)
            .expect("read should succeed");

        let decoded = decoded.expect("should decode to Some");
        if let Value::Map(map) = decoded {
            // Only the required field should be present
            assert_eq!(map.len(), 1);
            assert_eq!(
                map[0],
                (Value::Text("required_field".to_string()), Value::U32(42))
            );
        } else {
            panic!("expected Map");
        }
    }

    // -----------------------------------------------------------------------
    // encode_value_for_tree_keys() additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_for_tree_keys_u128() {
        let prop = DocumentPropertyType::U128;
        let val = Value::U128(42);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result.len(), 16);
        // Should match the static encode_u128
        assert_eq!(result, DocumentPropertyType::encode_u128(42));
    }

    #[test]
    fn test_encode_value_for_tree_keys_i128() {
        let prop = DocumentPropertyType::I128;
        let val = Value::I128(-42);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result.len(), 16);
        assert_eq!(result, DocumentPropertyType::encode_i128(-42));
    }

    #[test]
    fn test_encode_value_for_tree_keys_u64() {
        let prop = DocumentPropertyType::U64;
        let val = Value::U64(12345);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_u64(12345));
    }

    #[test]
    fn test_encode_value_for_tree_keys_i64() {
        let prop = DocumentPropertyType::I64;
        let val = Value::I64(-12345);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_i64(-12345));
    }

    #[test]
    fn test_encode_value_for_tree_keys_u32() {
        let prop = DocumentPropertyType::U32;
        let val = Value::U32(999);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_u32(999));
    }

    #[test]
    fn test_encode_value_for_tree_keys_i32() {
        let prop = DocumentPropertyType::I32;
        let val = Value::I32(-999);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_i32(-999));
    }

    #[test]
    fn test_encode_value_for_tree_keys_u16() {
        let prop = DocumentPropertyType::U16;
        let val = Value::U16(500);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_u16(500));
    }

    #[test]
    fn test_encode_value_for_tree_keys_i16() {
        let prop = DocumentPropertyType::I16;
        let val = Value::I16(-500);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_i16(-500));
    }

    #[test]
    fn test_encode_value_for_tree_keys_u8() {
        let prop = DocumentPropertyType::U8;
        let val = Value::U8(42);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_u8(42));
    }

    #[test]
    fn test_encode_value_for_tree_keys_i8() {
        let prop = DocumentPropertyType::I8;
        let val = Value::I8(-42);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_i8(-42));
    }

    #[test]
    fn test_encode_value_for_tree_keys_f64() {
        let prop = DocumentPropertyType::F64;
        let val = Value::Float(3.14);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(result, DocumentPropertyType::encode_float(3.14));
    }

    #[test]
    fn test_encode_value_for_tree_keys_date_timestamp() {
        let prop = DocumentPropertyType::Date;
        let val = Value::U64(1648910575000);
        let result = prop.encode_value_for_tree_keys(&val).unwrap();
        assert_eq!(
            result,
            DocumentPropertyType::encode_date_timestamp(1648910575000)
        );
    }

    #[test]
    fn test_encode_value_for_tree_keys_identifier() {
        let prop = DocumentPropertyType::Identifier;
        let id = [7u8; 32];
        let result = prop
            .encode_value_for_tree_keys(&Value::Identifier(id))
            .unwrap();
        assert_eq!(result, id.to_vec());
    }

    #[test]
    fn test_encode_value_for_tree_keys_variable_type_array_error() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let result = prop.encode_value_for_tree_keys(&Value::Array(vec![]));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // decode_value_for_tree_keys() additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_decode_value_for_tree_keys_u128() {
        let prop = DocumentPropertyType::U128;
        let encoded = DocumentPropertyType::encode_u128(42);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::U128(42));
    }

    #[test]
    fn test_decode_value_for_tree_keys_i128() {
        let prop = DocumentPropertyType::I128;
        let encoded = DocumentPropertyType::encode_i128(-42);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::I128(-42));
    }

    #[test]
    fn test_decode_value_for_tree_keys_u32() {
        let prop = DocumentPropertyType::U32;
        let encoded = DocumentPropertyType::encode_u32(999);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::U32(999));
    }

    #[test]
    fn test_decode_value_for_tree_keys_i32() {
        let prop = DocumentPropertyType::I32;
        let encoded = DocumentPropertyType::encode_i32(-999);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::I32(-999));
    }

    #[test]
    fn test_decode_value_for_tree_keys_u16() {
        let prop = DocumentPropertyType::U16;
        let encoded = DocumentPropertyType::encode_u16(500);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::U16(500));
    }

    #[test]
    fn test_decode_value_for_tree_keys_i16() {
        let prop = DocumentPropertyType::I16;
        let encoded = DocumentPropertyType::encode_i16(-500);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::I16(-500));
    }

    #[test]
    fn test_decode_value_for_tree_keys_u8() {
        let prop = DocumentPropertyType::U8;
        let encoded = DocumentPropertyType::encode_u8(42);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::U8(42));
    }

    #[test]
    fn test_decode_value_for_tree_keys_i8() {
        let prop = DocumentPropertyType::I8;
        let encoded = DocumentPropertyType::encode_i8(-42);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::I8(-42));
    }

    #[test]
    fn test_decode_value_for_tree_keys_f64() {
        let prop = DocumentPropertyType::F64;
        let encoded = DocumentPropertyType::encode_float(3.14);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        if let Value::Float(f) = decoded {
            assert!((f - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("expected float");
        }
    }

    #[test]
    fn test_decode_value_for_tree_keys_date() {
        let prop = DocumentPropertyType::Date;
        let encoded = DocumentPropertyType::encode_date_timestamp(1648910575000);
        let decoded = prop.decode_value_for_tree_keys(&encoded).unwrap();
        assert_eq!(decoded, Value::U64(1648910575000));
    }

    #[test]
    fn test_decode_value_for_tree_keys_identifier() {
        let prop = DocumentPropertyType::Identifier;
        let id = [7u8; 32];
        let decoded = prop.decode_value_for_tree_keys(&id).unwrap();
        if let Value::Identifier(decoded_id) = decoded {
            assert_eq!(decoded_id, id);
        } else {
            panic!("expected identifier");
        }
    }

    #[test]
    fn test_decode_value_for_tree_keys_variable_type_array_error() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let result = prop.decode_value_for_tree_keys(&[1, 2, 3]);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // tree keys roundtrip at boundary values
    // -----------------------------------------------------------------------

    #[test]
    fn test_tree_keys_roundtrip_u64_boundary_values() {
        let prop = DocumentPropertyType::U64;
        for val in [0u64, 1, u64::MAX / 2, u64::MAX] {
            let enc = prop.encode_value_for_tree_keys(&Value::U64(val)).unwrap();
            let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
            assert_eq!(dec, Value::U64(val));
        }
    }

    #[test]
    fn test_tree_keys_roundtrip_i64_boundary_values() {
        let prop = DocumentPropertyType::I64;
        for val in [i64::MIN, -1, 0, 1, i64::MAX] {
            let enc = prop.encode_value_for_tree_keys(&Value::I64(val)).unwrap();
            let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
            assert_eq!(dec, Value::I64(val));
        }
    }

    #[test]
    fn test_tree_keys_roundtrip_u128_boundary_values() {
        let prop = DocumentPropertyType::U128;
        for val in [0u128, 1, u128::MAX / 2, u128::MAX] {
            let enc = prop.encode_value_for_tree_keys(&Value::U128(val)).unwrap();
            let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
            assert_eq!(dec, Value::U128(val));
        }
    }

    #[test]
    fn test_tree_keys_roundtrip_i128_boundary_values() {
        let prop = DocumentPropertyType::I128;
        for val in [i128::MIN, -1, 0, 1, i128::MAX] {
            let enc = prop.encode_value_for_tree_keys(&Value::I128(val)).unwrap();
            let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
            assert_eq!(dec, Value::I128(val));
        }
    }

    #[test]
    fn test_tree_keys_roundtrip_string_empty() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let enc = prop
            .encode_value_for_tree_keys(&Value::Text("".to_string()))
            .unwrap();
        // Empty string should produce sentinel [0]
        assert_eq!(enc, vec![0]);
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, Value::Text("".to_string()));
    }

    #[test]
    fn test_tree_keys_roundtrip_string_nonempty() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let enc = prop
            .encode_value_for_tree_keys(&Value::Text("test".to_string()))
            .unwrap();
        let dec = prop.decode_value_for_tree_keys(&enc).unwrap();
        assert_eq!(dec, Value::Text("test".to_string()));
    }

    // -----------------------------------------------------------------------
    // read_optionally_from() additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_read_optionally_from_optional_u64_present() {
        // When required=false and marker byte is non-zero, the value follows
        let prop = DocumentPropertyType::U64;
        let mut data = vec![0xFF]; // marker: present
        data.extend_from_slice(&42u64.to_be_bytes());
        let mut reader = BufReader::new(data.as_slice());
        let (value, finished) = prop.read_optionally_from(&mut reader, false).unwrap();
        assert_eq!(value, Some(Value::U64(42)));
        assert!(!finished);
    }

    #[test]
    fn test_read_optionally_from_optional_i32_present() {
        let prop = DocumentPropertyType::I32;
        let mut data = vec![0xFF]; // marker: present
        data.extend_from_slice(&(-100i32).to_be_bytes());
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, false).unwrap();
        assert_eq!(value, Some(Value::I32(-100)));
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_20() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(20),
            max_size: Some(20),
        });
        let data = [42u8; 20];
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes20(data)));
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_36() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(36),
            max_size: Some(36),
        });
        let data = [99u8; 36];
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes36(data)));
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_non_special_size() {
        // A fixed-size byte array that is not 20, 32, or 36 should use Value::Bytes
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(10),
            max_size: Some(10),
        });
        let data = [1u8; 10];
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes(data.to_vec())));
    }

    #[test]
    fn test_read_optionally_from_byte_array_variable_empty() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(0),
            max_size: Some(100),
        });
        // varint 0 means zero-length byte array
        let data = 0usize.encode_var_vec();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        assert_eq!(value, Some(Value::Bytes(vec![])));
    }

    #[test]
    fn test_read_optionally_from_date_required() {
        let prop = DocumentPropertyType::Date;
        let data = 1648910575.0f64.to_be_bytes();
        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        if let Some(Value::Float(f)) = value {
            assert!((f - 1648910575.0).abs() < f64::EPSILON);
        } else {
            panic!("expected float for date");
        }
    }

    #[test]
    fn test_read_optionally_from_object_with_inner_fields() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "count".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);

        // Build the serialized object: varint(object_byte_len) + object_bytes
        let object_bytes = 100u32.to_be_bytes();
        let mut data = object_bytes.len().encode_var_vec();
        data.extend_from_slice(&object_bytes);

        let mut reader = BufReader::new(data.as_slice());
        let (value, _) = prop.read_optionally_from(&mut reader, true).unwrap();
        let value = value.expect("should decode object");
        if let Value::Map(map) = value {
            assert_eq!(map.len(), 1);
            assert_eq!(map[0], (Value::Text("count".to_string()), Value::U32(100)));
        } else {
            panic!("expected Map");
        }
    }

    // -----------------------------------------------------------------------
    // min_byte_size() / max_byte_size() additional tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_min_byte_size_object_sums_sub_fields() {
        let pv = PlatformVersion::latest();
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "a".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
            },
        );
        sub_fields.insert(
            "b".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U64,
                required: true,
                transient: false,
            },
        );
        let obj = DocumentPropertyType::Object(sub_fields);
        // 4 + 8 = 12
        assert_eq!(obj.min_byte_size(pv).unwrap(), Some(12));
    }

    #[test]
    fn test_max_byte_size_object_sums_sub_fields() {
        let pv = PlatformVersion::latest();
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "a".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U16,
                required: true,
                transient: false,
            },
        );
        sub_fields.insert(
            "b".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::Boolean,
                required: true,
                transient: false,
            },
        );
        let obj = DocumentPropertyType::Object(sub_fields);
        // 2 + 1 = 3
        assert_eq!(obj.max_byte_size(pv).unwrap(), Some(3));
    }

    #[test]
    fn test_min_byte_size_variable_type_array_returns_none() {
        let pv = PlatformVersion::latest();
        let vta = DocumentPropertyType::VariableTypeArray(vec![]);
        assert_eq!(vta.min_byte_size(pv).unwrap(), None);
    }

    #[test]
    fn test_max_byte_size_variable_type_array_returns_none() {
        let pv = PlatformVersion::latest();
        let vta = DocumentPropertyType::VariableTypeArray(vec![]);
        assert_eq!(vta.max_byte_size(pv).unwrap(), None);
    }

    #[test]
    fn test_min_byte_size_identifier() {
        let pv = PlatformVersion::latest();
        assert_eq!(
            DocumentPropertyType::Identifier.min_byte_size(pv).unwrap(),
            Some(32)
        );
    }

    #[test]
    fn test_max_byte_size_identifier() {
        let pv = PlatformVersion::latest();
        assert_eq!(
            DocumentPropertyType::Identifier.max_byte_size(pv).unwrap(),
            Some(32)
        );
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() marker byte verification for optional types
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_with_size_u32_not_required_has_marker() {
        let prop = DocumentPropertyType::U32;
        let result = prop.encode_value_with_size(Value::U32(100), false).unwrap();
        assert_eq!(result.len(), 5); // 1 marker + 4 bytes
        assert_eq!(result[0], 0xFF);
        assert_eq!(&result[1..], 100u32.to_be_bytes().as_slice());
    }

    #[test]
    fn test_encode_value_with_size_i32_not_required_has_marker() {
        let prop = DocumentPropertyType::I32;
        let result = prop.encode_value_with_size(Value::I32(-50), false).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], 0xFF);
    }

    #[test]
    fn test_encode_value_with_size_u16_not_required_has_marker() {
        let prop = DocumentPropertyType::U16;
        let result = prop.encode_value_with_size(Value::U16(300), false).unwrap();
        assert_eq!(result.len(), 3); // 1 marker + 2 bytes
        assert_eq!(result[0], 0xFF);
    }

    #[test]
    fn test_encode_value_with_size_i16_not_required_has_marker() {
        let prop = DocumentPropertyType::I16;
        let result = prop
            .encode_value_with_size(Value::I16(-100), false)
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 0xFF);
    }

    #[test]
    fn test_encode_value_with_size_u8_not_required_has_marker() {
        let prop = DocumentPropertyType::U8;
        let result = prop.encode_value_with_size(Value::U8(42), false).unwrap();
        assert_eq!(result.len(), 2); // 1 marker + 1 byte
        assert_eq!(result[0], 0xFF);
        assert_eq!(result[1], 42);
    }

    #[test]
    fn test_encode_value_with_size_i8_not_required_has_marker() {
        let prop = DocumentPropertyType::I8;
        let result = prop.encode_value_with_size(Value::I8(-10), false).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 0xFF);
    }

    #[test]
    fn test_encode_value_with_size_i128_not_required_has_marker() {
        let prop = DocumentPropertyType::I128;
        let result = prop
            .encode_value_with_size(Value::I128(-500), false)
            .unwrap();
        assert_eq!(result.len(), 17); // 1 marker + 16 bytes
        assert_eq!(result[0], 0xFF);
    }

    // -----------------------------------------------------------------------
    // random_value() tests - exercise branches not covered elsewhere
    // -----------------------------------------------------------------------

    use rand::SeedableRng;

    #[test]
    fn test_random_value_produces_expected_type_for_all_scalar_variants() {
        let mut rng = StdRng::seed_from_u64(1);
        assert!(matches!(
            DocumentPropertyType::U128.random_value(&mut rng),
            Value::U128(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I128.random_value(&mut rng),
            Value::I128(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U64.random_value(&mut rng),
            Value::U64(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I64.random_value(&mut rng),
            Value::I64(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U32.random_value(&mut rng),
            Value::U32(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I32.random_value(&mut rng),
            Value::I32(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U16.random_value(&mut rng),
            Value::U16(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I16.random_value(&mut rng),
            Value::I16(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U8.random_value(&mut rng),
            Value::U8(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I8.random_value(&mut rng),
            Value::I8(_)
        ));
        assert!(matches!(
            DocumentPropertyType::F64.random_value(&mut rng),
            Value::Float(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Boolean.random_value(&mut rng),
            Value::Bool(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Date.random_value(&mut rng),
            Value::Float(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Identifier.random_value(&mut rng),
            Value::Identifier(_)
        ));
    }

    #[test]
    fn test_random_value_string_respects_size_bounds() {
        let mut rng = StdRng::seed_from_u64(2);
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(5),
            max_length: Some(10),
        });
        // Exercise several random draws
        for _ in 0..5 {
            if let Value::Text(s) = prop.random_value(&mut rng) {
                assert!(
                    s.len() >= 5 && s.len() <= 10,
                    "length out of range: {}",
                    s.len()
                );
                assert!(s.chars().all(|c| c.is_ascii_alphanumeric()));
            } else {
                panic!("expected Text variant");
            }
        }
    }

    #[test]
    fn test_random_value_byte_array_fixed_size_20() {
        let mut rng = StdRng::seed_from_u64(3);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(20),
            max_size: Some(20),
        });
        // min == max == 20 => Value::Bytes20 specialization
        match prop.random_value(&mut rng) {
            Value::Bytes20(_) => {}
            v => panic!("expected Bytes20, got {:?}", v),
        }
    }

    #[test]
    fn test_random_value_byte_array_fixed_size_32() {
        let mut rng = StdRng::seed_from_u64(4);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(32),
            max_size: Some(32),
        });
        // min == max == 32 => Value::Bytes32 specialization
        match prop.random_value(&mut rng) {
            Value::Bytes32(_) => {}
            v => panic!("expected Bytes32, got {:?}", v),
        }
    }

    #[test]
    fn test_random_value_byte_array_fixed_size_36() {
        let mut rng = StdRng::seed_from_u64(5);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(36),
            max_size: Some(36),
        });
        // min == max == 36 => Value::Bytes36 specialization
        match prop.random_value(&mut rng) {
            Value::Bytes36(_) => {}
            v => panic!("expected Bytes36, got {:?}", v),
        }
    }

    #[test]
    fn test_random_value_byte_array_fixed_size_other_uses_bytes() {
        let mut rng = StdRng::seed_from_u64(6);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(8),
            max_size: Some(8),
        });
        // min == max but not in the special {20, 32, 36} set => Value::Bytes
        match prop.random_value(&mut rng) {
            Value::Bytes(b) => assert_eq!(b.len(), 8),
            v => panic!("expected Bytes, got {:?}", v),
        }
    }

    #[test]
    fn test_random_value_byte_array_variable_uses_bytes() {
        let mut rng = StdRng::seed_from_u64(7);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(1),
            max_size: Some(10),
        });
        // min != max => Value::Bytes (never Bytes20/32/36)
        for _ in 0..5 {
            match prop.random_value(&mut rng) {
                Value::Bytes(b) => {
                    assert!(!b.is_empty() && b.len() <= 10);
                }
                v => panic!("expected Bytes, got {:?}", v),
            }
        }
    }

    #[test]
    fn test_random_value_array_and_variable_type_array_return_null() {
        let mut rng = StdRng::seed_from_u64(8);
        assert_eq!(
            DocumentPropertyType::Array(ArrayItemType::Integer).random_value(&mut rng),
            Value::Null
        );
        assert_eq!(
            DocumentPropertyType::VariableTypeArray(vec![]).random_value(&mut rng),
            Value::Null
        );
    }

    #[test]
    fn test_random_value_object_only_includes_required_fields() {
        let mut rng = StdRng::seed_from_u64(9);
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "req".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
            },
        );
        sub_fields.insert(
            "opt".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U64,
                required: false,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(sub_fields);
        let val = prop.random_value(&mut rng);
        if let Value::Map(entries) = val {
            assert_eq!(
                entries.len(),
                1,
                "only the required field should be present"
            );
            assert_eq!(entries[0].0, Value::Text("req".to_string()));
            assert!(matches!(entries[0].1, Value::U32(_)));
        } else {
            panic!("expected Map");
        }
    }

    // -----------------------------------------------------------------------
    // random_sub_filled_value() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_random_sub_filled_value_string_uses_min_size() {
        let mut rng = StdRng::seed_from_u64(10);
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(7),
            max_length: Some(20),
        });
        if let Value::Text(s) = prop.random_sub_filled_value(&mut rng) {
            assert_eq!(s.len(), 7);
        } else {
            panic!("expected Text");
        }
    }

    #[test]
    fn test_random_sub_filled_value_byte_array_uses_min_size() {
        let mut rng = StdRng::seed_from_u64(11);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(4),
            max_size: Some(100),
        });
        if let Value::Bytes(b) = prop.random_sub_filled_value(&mut rng) {
            assert_eq!(b.len(), 4);
        } else {
            panic!("expected Bytes");
        }
    }

    #[test]
    fn test_random_sub_filled_value_object_includes_all_fields() {
        let mut rng = StdRng::seed_from_u64(12);
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "req".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
            },
        );
        sub_fields.insert(
            "opt".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U64,
                required: false,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(sub_fields);
        // sub_filled_value includes ALL fields regardless of required flag
        let val = prop.random_sub_filled_value(&mut rng);
        if let Value::Map(entries) = val {
            assert_eq!(entries.len(), 2);
        } else {
            panic!("expected Map");
        }
    }

    #[test]
    fn test_random_sub_filled_value_array_returns_null() {
        let mut rng = StdRng::seed_from_u64(13);
        assert_eq!(
            DocumentPropertyType::Array(ArrayItemType::Integer).random_sub_filled_value(&mut rng),
            Value::Null
        );
        assert_eq!(
            DocumentPropertyType::VariableTypeArray(vec![]).random_sub_filled_value(&mut rng),
            Value::Null
        );
    }

    #[test]
    fn test_random_sub_filled_value_date_returns_float() {
        let mut rng = StdRng::seed_from_u64(14);
        assert!(matches!(
            DocumentPropertyType::Date.random_sub_filled_value(&mut rng),
            Value::Float(_)
        ));
    }

    #[test]
    fn test_random_sub_filled_value_identifier() {
        let mut rng = StdRng::seed_from_u64(15);
        assert!(matches!(
            DocumentPropertyType::Identifier.random_sub_filled_value(&mut rng),
            Value::Identifier(_)
        ));
    }

    // -----------------------------------------------------------------------
    // random_filled_value() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_random_filled_value_string_uses_max_size() {
        let mut rng = StdRng::seed_from_u64(16);
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(1),
            max_length: Some(12),
        });
        if let Value::Text(s) = prop.random_filled_value(&mut rng) {
            assert_eq!(s.len(), 12);
        } else {
            panic!("expected Text");
        }
    }

    #[test]
    fn test_random_filled_value_byte_array_uses_max_size() {
        let mut rng = StdRng::seed_from_u64(17);
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(0),
            max_size: Some(9),
        });
        if let Value::Bytes(b) = prop.random_filled_value(&mut rng) {
            assert_eq!(b.len(), 9);
        } else {
            panic!("expected Bytes");
        }
    }

    #[test]
    fn test_random_filled_value_object_includes_all_fields() {
        let mut rng = StdRng::seed_from_u64(18);
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "a".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U8,
                required: true,
                transient: false,
            },
        );
        sub_fields.insert(
            "b".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::Boolean,
                required: false,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(sub_fields);
        let val = prop.random_filled_value(&mut rng);
        if let Value::Map(entries) = val {
            assert_eq!(entries.len(), 2);
        } else {
            panic!("expected Map");
        }
    }

    #[test]
    fn test_random_filled_value_scalars() {
        let mut rng = StdRng::seed_from_u64(19);
        // exhaustively exercise each scalar variant not already covered
        assert!(matches!(
            DocumentPropertyType::U128.random_filled_value(&mut rng),
            Value::U128(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I128.random_filled_value(&mut rng),
            Value::I128(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U64.random_filled_value(&mut rng),
            Value::U64(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I64.random_filled_value(&mut rng),
            Value::I64(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U32.random_filled_value(&mut rng),
            Value::U32(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I32.random_filled_value(&mut rng),
            Value::I32(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U16.random_filled_value(&mut rng),
            Value::U16(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I16.random_filled_value(&mut rng),
            Value::I16(_)
        ));
        assert!(matches!(
            DocumentPropertyType::U8.random_filled_value(&mut rng),
            Value::U8(_)
        ));
        assert!(matches!(
            DocumentPropertyType::I8.random_filled_value(&mut rng),
            Value::I8(_)
        ));
        assert!(matches!(
            DocumentPropertyType::F64.random_filled_value(&mut rng),
            Value::Float(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Boolean.random_filled_value(&mut rng),
            Value::Bool(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Date.random_filled_value(&mut rng),
            Value::Float(_)
        ));
        assert!(matches!(
            DocumentPropertyType::Identifier.random_filled_value(&mut rng),
            Value::Identifier(_)
        ));
        assert_eq!(
            DocumentPropertyType::Array(ArrayItemType::Integer).random_filled_value(&mut rng),
            Value::Null
        );
        assert_eq!(
            DocumentPropertyType::VariableTypeArray(vec![]).random_filled_value(&mut rng),
            Value::Null
        );
    }

    #[test]
    fn test_random_size_respects_range() {
        let mut rng = StdRng::seed_from_u64(20);
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(3),
            max_length: Some(6),
        });
        for _ in 0..10 {
            let sz = prop.random_size(&mut rng);
            assert!((3..=6).contains(&sz));
        }
    }

    // -----------------------------------------------------------------------
    // read_optionally_from() corrupted / truncated buffer error paths
    // -----------------------------------------------------------------------

    #[test]
    fn test_read_optionally_from_u64_truncated_returns_error() {
        // Only 3 bytes but u64 needs 8
        let prop = DocumentPropertyType::U64;
        let data: &[u8] = &[0, 0, 0];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_i64_truncated_returns_error() {
        let prop = DocumentPropertyType::I64;
        let data: &[u8] = &[1, 2];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_u128_truncated_returns_error() {
        let prop = DocumentPropertyType::U128;
        let data: &[u8] = &[0; 4];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_i128_truncated_returns_error() {
        let prop = DocumentPropertyType::I128;
        let data: &[u8] = &[0; 2];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_u32_truncated_returns_error() {
        let prop = DocumentPropertyType::U32;
        let data: &[u8] = &[0];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_i32_truncated_returns_error() {
        let prop = DocumentPropertyType::I32;
        let data: &[u8] = &[0, 1];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_u16_truncated_returns_error() {
        let prop = DocumentPropertyType::U16;
        let data: &[u8] = &[];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_i16_truncated_returns_error() {
        let prop = DocumentPropertyType::I16;
        let data: &[u8] = &[7];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_u8_eof_returns_error() {
        // required=true but empty buffer: u8 read must fail
        let prop = DocumentPropertyType::U8;
        let data: &[u8] = &[];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_i8_eof_returns_error() {
        let prop = DocumentPropertyType::I8;
        let data: &[u8] = &[];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_f64_truncated_returns_error() {
        let prop = DocumentPropertyType::F64;
        let data: &[u8] = &[0, 0, 0];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_date_truncated_returns_error() {
        let prop = DocumentPropertyType::Date;
        let data: &[u8] = &[1, 2];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_boolean_eof_returns_error() {
        let prop = DocumentPropertyType::Boolean;
        let data: &[u8] = &[];
        let mut reader = BufReader::new(data);
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_identifier_truncated_returns_error() {
        let prop = DocumentPropertyType::Identifier;
        // Only 16 bytes but identifier needs 32
        let data = [1u8; 16];
        let mut reader = BufReader::new(data.as_slice());
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_string_invalid_utf8_returns_error() {
        use integer_encoding::VarInt;
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        // Valid varint length but invalid UTF-8 bytes
        let invalid_bytes = vec![0xFFu8, 0xFEu8, 0xFDu8];
        let mut data = invalid_bytes.len().encode_var_vec();
        data.extend_from_slice(&invalid_bytes);
        let mut reader = BufReader::new(data.as_slice());
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_string_truncated_returns_error() {
        use integer_encoding::VarInt;
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        // varint says 10 bytes follow, but only provide 2
        let mut data = 10usize.encode_var_vec();
        data.push(b'a');
        data.push(b'b');
        let mut reader = BufReader::new(data.as_slice());
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_byte_array_fixed_size_truncated_returns_error() {
        // min == max == 32, but provide only 10 bytes
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(32),
            max_size: Some(32),
        });
        let data = [1u8; 10];
        let mut reader = BufReader::new(data.as_slice());
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_object_truncated_length_returns_error() {
        use integer_encoding::VarInt;
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "x".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);
        // Claim 100 bytes follow but provide only 2
        let mut data = 100usize.encode_var_vec();
        data.push(0);
        data.push(0);
        let mut reader = BufReader::new(data.as_slice());
        assert!(prop.read_optionally_from(&mut reader, true).is_err());
    }

    #[test]
    fn test_read_optionally_from_object_required_field_after_finished_buffer() {
        // Exercises the explicit "required field after finished buffer in object"
        // branch: the optional first field exhausts the inner buffer by reading
        // an absence marker from a zero-length buffer (which flips
        // `finished_buffer` to true), then the iterator sees a required field
        // with the buffer already finished and must produce a
        // CorruptedSerialization error.
        use integer_encoding::VarInt;
        let mut inner_fields = IndexMap::new();
        // First field is optional
        inner_fields.insert(
            "a".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: false,
                transient: false,
            },
        );
        // Second field is required
        inner_fields.insert(
            "b".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: true,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);

        // Empty inner buffer: the optional-field read observes EOF on the
        // absence-marker byte, returns (None, finished = true). On the next
        // iteration, "b" is required and the buffer is finished => the
        // targeted error branch fires.
        let inner_bytes: Vec<u8> = vec![];
        let mut data = inner_bytes.len().encode_var_vec();
        data.extend_from_slice(&inner_bytes);
        let mut reader = BufReader::new(data.as_slice());
        let err = prop
            .read_optionally_from(&mut reader, true)
            .expect_err("required field with finished buffer must error");
        match err {
            DataContractError::CorruptedSerialization(msg) => {
                assert!(
                    msg.contains("required field after finished buffer in object"),
                    "expected the finished-buffer branch, got: {msg}"
                );
            }
            other => panic!("expected CorruptedSerialization, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // encode_value_with_size() type-mismatch errors
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_with_size_boolean_type_mismatch() {
        let prop = DocumentPropertyType::Boolean;
        // U64 cannot be coerced to bool
        let result = prop.encode_value_with_size(Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_object_type_mismatch() {
        let prop = DocumentPropertyType::Object(IndexMap::new());
        let result = prop.encode_value_with_size(Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_array_type_mismatch() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        // Not an Array value
        let result = prop.encode_value_with_size(Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_f64_type_mismatch() {
        let prop = DocumentPropertyType::F64;
        // Text cannot be converted to a float
        let result = prop.encode_value_with_size(Value::Text("not a number".to_string()), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_u64_type_mismatch() {
        let prop = DocumentPropertyType::U64;
        let result = prop.encode_value_with_size(Value::Text("x".to_string()), true);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // encode_value_ref_with_size() type-mismatch errors
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_ref_with_size_string_type_mismatch() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
        });
        let result = prop.encode_value_ref_with_size(&Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_boolean_type_mismatch() {
        let prop = DocumentPropertyType::Boolean;
        let result = prop.encode_value_ref_with_size(&Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_object_type_mismatch() {
        let prop = DocumentPropertyType::Object(IndexMap::new());
        let result = prop.encode_value_ref_with_size(&Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_array_type_mismatch() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let result = prop.encode_value_ref_with_size(&Value::U64(1), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_object_missing_required_field_errors() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "name".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::String(StringPropertySizes {
                    min_length: None,
                    max_length: Some(100),
                }),
                required: true,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);
        let val = Value::Map(vec![]);
        let result = prop.encode_value_ref_with_size(&val, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_object_optional_absent_pushes_zero() {
        let mut inner_fields = IndexMap::new();
        inner_fields.insert(
            "opt".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U32,
                required: false,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(inner_fields);
        // Missing optional field path encodes a 0 absence marker
        let val = Value::Map(vec![]);
        let encoded = prop.encode_value_ref_with_size(&val, true).unwrap();
        // The body is one byte (0), prefixed with varint length (1).
        assert_eq!(encoded, vec![1, 0]);
    }

    #[test]
    fn test_encode_value_ref_with_size_variable_type_array_returns_error_specific() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![]);
        let val = Value::Array(vec![]);
        let result = prop.encode_value_ref_with_size(&val, true);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Array encode roundtrip (covers array arm of encode_value_with_size and
    // encode_value_ref_with_size)
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_with_size_array_of_integers() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let val = Value::Array(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
        let result = prop.encode_value_with_size(val, true).unwrap();
        // varint(3) + 3 * 8 bytes
        assert_eq!(result.len(), 1 + 3 * 8);
        assert_eq!(result[0], 3);
    }

    #[test]
    fn test_encode_value_ref_with_size_array_of_integers() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        let val = Value::Array(vec![Value::I64(1), Value::I64(2)]);
        let result = prop.encode_value_ref_with_size(&val, true).unwrap();
        // varint(2) + 2 * 8 bytes
        assert_eq!(result.len(), 1 + 2 * 8);
        assert_eq!(result[0], 2);
    }

    // -----------------------------------------------------------------------
    // try_from_value_map() - extra branches
    // -----------------------------------------------------------------------

    #[test]
    fn test_try_from_value_map_string_without_sizes() {
        let type_val = Value::Text("string".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(
            result,
            DocumentPropertyType::String(StringPropertySizes {
                min_length: None,
                max_length: None,
            })
        );
    }

    #[test]
    fn test_try_from_value_map_object_type() {
        let type_val = Value::Text("object".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert!(matches!(result, DocumentPropertyType::Object(_)));
    }

    #[test]
    fn test_try_from_value_map_integer_only_min_positive() {
        // sized, only min >= 0 => U64
        let type_val = Value::Text("integer".to_string());
        let min_val = Value::I64(10);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("minimum".to_string(), &min_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::U64);
    }

    #[test]
    fn test_try_from_value_map_integer_only_min_negative() {
        // sized, only min < 0 => I64
        let type_val = Value::Text("integer".to_string());
        let min_val = Value::I64(-10);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("minimum".to_string(), &min_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::I64);
    }

    #[test]
    fn test_try_from_value_map_integer_only_max() {
        // sized, only max <= u8::MAX => U8
        let type_val = Value::Text("integer".to_string());
        let max_val = Value::I64(200);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("maximum".to_string(), &max_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::U8);
    }

    #[test]
    fn test_try_from_value_map_integer_no_min_no_max_defaults_to_i64() {
        // sized, no min/max, no enum => I64
        let type_val = Value::Text("integer".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert_eq!(result, DocumentPropertyType::I64);
    }

    #[test]
    fn test_try_from_value_map_integer_with_enum_min_max() {
        // sized, enum values drive the integer selection
        let type_val = Value::Text("integer".to_string());
        let enum_val = Value::Array(vec![Value::I64(0), Value::I64(1), Value::I64(255)]);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("enum".to_string(), &enum_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        // min=0, max=255 => U8
        assert_eq!(result, DocumentPropertyType::U8);
    }

    #[test]
    fn test_try_from_value_map_integer_with_enum_single_value() {
        // A single-element enum picks the unsigned type for that max
        let type_val = Value::Text("integer".to_string());
        let enum_val = Value::Array(vec![Value::I64(300)]);
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("enum".to_string(), &enum_val);
        let options = DocumentPropertyTypeParsingOptions {
            sized_integer_types: true,
        };
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        // 300 => U16
        assert_eq!(result, DocumentPropertyType::U16);
    }

    #[test]
    fn test_try_from_value_map_array_byte_array_non_identifier_media_type() {
        // Non-identifier content-media-type => falls through to ByteArray
        let type_val = Value::Text("array".to_string());
        let byte_array_val = Value::Bool(true);
        let media_type_val = Value::Text("application/octet-stream".to_string());
        let mut map = BTreeMap::new();
        map.insert("type".to_string(), &type_val);
        map.insert("byteArray".to_string(), &byte_array_val);
        map.insert("contentMediaType".to_string(), &media_type_val);
        let options = DocumentPropertyTypeParsingOptions::default();
        let result = DocumentPropertyType::try_from_value_map(&map, &options).unwrap();
        assert!(matches!(result, DocumentPropertyType::ByteArray(_)));
    }

    // -----------------------------------------------------------------------
    // find_integer_type_for_min_and_max_values() - negative boundaries
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_integer_type_negative_small_range_is_i8() {
        assert_eq!(
            find_integer_type_for_min_and_max_values(-50, 50),
            DocumentPropertyType::I8
        );
    }

    #[test]
    fn test_find_integer_type_negative_medium_range_is_i16() {
        assert_eq!(
            find_integer_type_for_min_and_max_values(-1000, 1000),
            DocumentPropertyType::I16
        );
    }

    #[test]
    fn test_find_integer_type_negative_large_range_is_i32() {
        assert_eq!(
            find_integer_type_for_min_and_max_values(-100_000, 100_000),
            DocumentPropertyType::I32
        );
    }

    #[test]
    fn test_find_integer_type_very_large_negative_is_i64() {
        assert_eq!(
            find_integer_type_for_min_and_max_values(i64::MIN, 0),
            DocumentPropertyType::I64
        );
    }

    // -----------------------------------------------------------------------
    // sanitize_value_mut() - additional branches
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanitize_value_mut_byte_array_from_base64_fallback() {
        // The hex decode should fail (contains +/= padding); base64 path should win
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        // "hello" in base64 is "aGVsbG8="
        let mut val = Value::Text("aGVsbG8=".to_string());
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Bytes(b"hello".to_vec()));
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_size_constraint_rejects() {
        // hex of 10 bytes, but min_size is 100 => value is left unchanged
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(100),
            max_size: None,
        });
        let original = Value::Text("aabbccddee".to_string()); // 5 bytes
        let mut val = original.clone();
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, original, "out-of-bounds byte array must remain text");
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_fixed_size_32() {
        // Fixed 32-byte hex string => Value::Bytes32 specialization
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(32),
            max_size: Some(32),
        });
        // 64 hex chars = 32 bytes
        let hex_str = "00".repeat(32);
        let mut val = Value::Text(hex_str);
        prop.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes32(_)));
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_fixed_size_20() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(20),
            max_size: Some(20),
        });
        let hex_str = "ab".repeat(20);
        let mut val = Value::Text(hex_str);
        prop.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes20(_)));
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_fixed_size_36() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(36),
            max_size: Some(36),
        });
        let hex_str = "cd".repeat(36);
        let mut val = Value::Text(hex_str);
        prop.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes36(_)));
    }

    #[test]
    fn test_sanitize_value_mut_byte_array_undecodable_leaves_unchanged() {
        // Neither valid hex (odd chars) nor valid base64 (has !@# chars)
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: None,
            max_size: None,
        });
        let original = Value::Text("!@#not valid!".to_string());
        let mut val = original.clone();
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, original);
    }

    #[test]
    fn test_sanitize_value_mut_object_nested() {
        // Object sanitization should recurse into nested fields
        let mut sub_fields = IndexMap::new();
        sub_fields.insert(
            "small".to_string(),
            DocumentProperty {
                property_type: DocumentPropertyType::U8,
                required: true,
                transient: false,
            },
        );
        let prop = DocumentPropertyType::Object(sub_fields);

        let mut val = Value::Map(vec![(
            Value::Text("small".to_string()),
            Value::U32(200), // will be sanitized to U8
        )]);
        prop.sanitize_value_mut(&mut val);
        if let Value::Map(entries) = val {
            assert_eq!(entries[0].1, Value::U8(200));
        } else {
            panic!("expected Map");
        }
    }

    #[test]
    fn test_sanitize_value_mut_array_elements() {
        let prop = DocumentPropertyType::Array(ArrayItemType::Integer);
        // Provide an array of Values
        let original_vals = vec![Value::I64(1), Value::I64(2)];
        let mut val = Value::Array(original_vals.clone());
        prop.sanitize_value_mut(&mut val);
        // Array path iterates every element; item_type.sanitize_value_mut is
        // defined in array.rs and shouldn't panic on well-formed input.
        if let Value::Array(items) = val {
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_sanitize_value_mut_variable_type_array_elements() {
        let prop = DocumentPropertyType::VariableTypeArray(vec![
            ArrayItemType::Integer,
            ArrayItemType::Integer,
        ]);
        let mut val = Value::Array(vec![Value::I64(10), Value::I64(20)]);
        prop.sanitize_value_mut(&mut val);
        if let Value::Array(items) = val {
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_sanitize_value_mut_u128_already_correct_unchanged() {
        let prop = DocumentPropertyType::U128;
        let mut val = Value::U128(42);
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U128(42));
    }

    #[test]
    fn test_sanitize_value_mut_u8_out_of_range_unchanged() {
        let prop = DocumentPropertyType::U8;
        let original = Value::U16(300); // > u8::MAX
        let mut val = original.clone();
        prop.sanitize_value_mut(&mut val);
        // Guard clause `n <= u8::MAX as u16` fails, so no conversion
        assert_eq!(val, original);
    }

    #[test]
    fn test_sanitize_value_mut_i8_out_of_range_unchanged() {
        let prop = DocumentPropertyType::I8;
        let original = Value::I16(500);
        let mut val = original.clone();
        prop.sanitize_value_mut(&mut val);
        assert_eq!(val, original);
    }

    // -----------------------------------------------------------------------
    // Additional numeric encode/decode roundtrips at boundaries through
    // value_from_string()
    // -----------------------------------------------------------------------

    #[test]
    fn test_value_from_string_i64_min_max() {
        let prop = DocumentPropertyType::I64;
        let min_str = i64::MIN.to_string();
        let max_str = i64::MAX.to_string();
        assert_eq!(
            prop.value_from_string(&min_str).unwrap(),
            Value::I64(i64::MIN)
        );
        assert_eq!(
            prop.value_from_string(&max_str).unwrap(),
            Value::I64(i64::MAX)
        );
    }

    #[test]
    fn test_value_from_string_u128_overflow_errors() {
        let prop = DocumentPropertyType::U128;
        // One larger than u128::MAX
        let out_of_range = "340282366920938463463374607431768211456";
        assert!(prop.value_from_string(out_of_range).is_err());
    }

    #[test]
    fn test_value_from_string_i128_overflow_errors() {
        let prop = DocumentPropertyType::I128;
        // Way too small
        let out_of_range = "-170141183460469231731687303715884105729";
        assert!(prop.value_from_string(out_of_range).is_err());
    }

    #[test]
    fn test_value_from_string_u8_negative_errors() {
        let prop = DocumentPropertyType::U8;
        assert!(prop.value_from_string("-1").is_err());
    }

    #[test]
    fn test_value_from_string_f64_invalid_errors() {
        let prop = DocumentPropertyType::F64;
        assert!(prop.value_from_string("not_a_float").is_err());
    }

    #[test]
    fn test_value_from_string_boolean_invalid_empty() {
        let prop = DocumentPropertyType::Boolean;
        assert!(prop.value_from_string("").is_err());
    }

    #[test]
    fn test_value_from_string_string_at_exact_max_len_ok() {
        let prop = DocumentPropertyType::String(StringPropertySizes {
            min_length: Some(3),
            max_length: Some(5),
        });
        // Boundary: exactly min and exactly max
        assert!(prop.value_from_string("abc").is_ok());
        assert!(prop.value_from_string("abcde").is_ok());
    }

    #[test]
    fn test_value_from_string_byte_array_exact_boundaries() {
        let prop = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(2),
            max_size: Some(4),
        });
        // 2 hex chars = 1 byte -> too small
        assert!(prop.value_from_string("ab").is_err());
        // 4 hex chars = 2 bytes -> ok
        assert!(prop.value_from_string("abcd").is_ok());
        // 8 hex chars = 4 bytes -> ok
        assert!(prop.value_from_string("aabbccdd").is_ok());
        // 10 hex chars = 5 bytes -> too big
        assert!(prop.value_from_string("aabbccddee").is_err());
    }

    // -----------------------------------------------------------------------
    // DocumentPropertyTypeParsingOptions::From<&DataContractConfig> test
    // -----------------------------------------------------------------------

    #[test]
    fn test_parsing_options_from_data_contract_config() {
        let config = DataContractConfig::default_for_version(PlatformVersion::latest())
            .expect("should create default config");
        let opts: DocumentPropertyTypeParsingOptions = (&config).into();
        // Just verify that the conversion yields the same sized_integer_types
        assert_eq!(opts.sized_integer_types, config.sized_integer_types());
    }

    // -----------------------------------------------------------------------
    // get_field_type_matching_error() - producer of ValueWrongType
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_field_type_matching_error_is_value_wrong_type() {
        let err = get_field_type_matching_error(&Value::U64(1));
        match err {
            DataContractError::ValueWrongType(msg) => {
                assert!(msg.contains("document field type"));
            }
            other => panic!("expected ValueWrongType, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Integer encode/decode roundtrips for u128 boundary values
    // -----------------------------------------------------------------------

    #[test]
    fn test_decode_i128_of_zero_roundtrip() {
        let enc = DocumentPropertyType::encode_i128(0);
        assert_eq!(DocumentPropertyType::decode_i128(&enc).unwrap(), 0);
    }

    #[test]
    fn test_encode_i128_preserves_sort_order() {
        let values: Vec<i128> = vec![i128::MIN, -100, -1, 0, 1, 100, i128::MAX];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_i128(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for i128");
        }
    }

    #[test]
    fn test_encode_u128_preserves_sort_order() {
        // sort order holds in the lower half of the u128 range
        let values: Vec<u128> = vec![0, 1, 100, 1_000_000, i128::MAX as u128];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_u128(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for u128");
        }
    }

    #[test]
    fn test_encode_u32_preserves_sort_order() {
        let values: Vec<u32> = vec![0, 1, 100, 1_000, i32::MAX as u32];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_u32(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1], "sort order not preserved for u32");
        }
    }

    #[test]
    fn test_encode_i16_preserves_sort_order() {
        let values: Vec<i16> = vec![i16::MIN, -1000, -1, 0, 1, 1000, i16::MAX];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_i16(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn test_encode_i8_preserves_sort_order() {
        let values: Vec<i8> = vec![i8::MIN, -100, -1, 0, 1, 100, i8::MAX];
        let encoded: Vec<Vec<u8>> = values
            .iter()
            .map(|v| DocumentPropertyType::encode_i8(*v))
            .collect();
        for window in encoded.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn should_serialize_reference_metadata() {
        let property = DocumentProperty {
            property_type: DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Identity,
            ),
            required: false,
            transient: false,
        };

        let value = serde_json::to_value(&property).expect("serialization should succeed");

        assert_eq!(
            value.get("property_type"),
            Some(&serde_json::json!({
                "IdentifierWithReference": "identity"
            }))
        );
    }

    #[test]
    fn should_display_reference_targets() {
        let contract_id = Identifier::from([7u8; 32]);

        assert_eq!(
            DocumentPropertyReferenceTarget::Identity.to_string(),
            "identity"
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::Contract.to_string(),
            "contract"
        );
        assert_eq!(DocumentPropertyReferenceTarget::Token.to_string(), "token");
        assert_eq!(
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: Some(contract_id),
                document_type_name: "note".to_string(),
            }
            .to_string(),
            format!("permanent document (contract {contract_id}, document type note)")
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: None,
                document_type_name: "note".to_string(),
            }
            .to_string(),
            "permanent document (own contract, document type note)"
        );
    }
}
