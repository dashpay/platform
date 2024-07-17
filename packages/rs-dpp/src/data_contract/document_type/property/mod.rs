use std::convert::TryInto;

use std::io::{BufReader, Cursor, Read};

use crate::data_contract::errors::DataContractError;

use crate::consensus::basic::decode::DecodingError;
use crate::prelude::TimestampMillis;
use crate::ProtocolError;
use array::ArrayItemType;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use integer_encoding::{VarInt, VarIntReader};
use platform_value::{Identifier, Value};
use rand::distributions::{Alphanumeric, Standard};
use rand::rngs::StdRng;
use rand::Rng;
use serde::Serialize;

pub mod array;

// This struct will be changed in future to support more validation logic and serialization
// It will become versioned and it will be introduced by a new document type version
// @append_only
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct DocumentProperty {
    pub property_type: DocumentPropertyType,
    pub required: bool,
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
}

impl DocumentPropertyType {
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
            DocumentPropertyType::Identifier => "identifier".to_string(),
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
            DocumentPropertyType::Identifier => Some(32),
        }
    }

    pub fn min_byte_size(&self) -> Option<u16> {
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
                Some(size) => Some(size * 4),
            },
            DocumentPropertyType::ByteArray(sizes) => match sizes.min_size {
                None => Some(0),
                Some(size) => Some(size),
            },
            DocumentPropertyType::Boolean => Some(1),
            DocumentPropertyType::Date => Some(8),
            DocumentPropertyType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.property_type.min_byte_size())
                .sum(),
            DocumentPropertyType::Array(_) => None,
            DocumentPropertyType::VariableTypeArray(_) => None,
            DocumentPropertyType::Identifier => Some(32),
        }
    }

    pub fn max_byte_size(&self) -> Option<u16> {
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
                None => Some(u16::MAX),
                Some(size) => Some(size * 4),
            },
            DocumentPropertyType::ByteArray(sizes) => match sizes.max_size {
                None => Some(u16::MAX),
                Some(size) => Some(size),
            },
            DocumentPropertyType::Boolean => Some(1),
            DocumentPropertyType::Date => Some(8),
            DocumentPropertyType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.property_type.max_byte_size())
                .sum(),
            DocumentPropertyType::Array(_) => None,
            DocumentPropertyType::VariableTypeArray(_) => None,
            DocumentPropertyType::Identifier => Some(32),
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
            DocumentPropertyType::Identifier => Some(32),
        }
    }

    /// The middle size rounded down halfway between min and max size
    pub fn middle_size(&self) -> Option<u16> {
        match self {
            DocumentPropertyType::Array(_) | DocumentPropertyType::VariableTypeArray(_) => {
                return None
            }
            _ => {}
        }
        let min_size = self.min_size().unwrap();
        let max_size = self.max_size().unwrap();
        Some((min_size + max_size) / 2)
    }

    /// The middle size rounded up halfway between min and max size
    pub fn middle_size_ceil(&self) -> Option<u16> {
        match self {
            DocumentPropertyType::Array(_) | DocumentPropertyType::VariableTypeArray(_) => {
                return None
            }
            _ => {}
        }
        let min_size = self.min_size().unwrap();
        let max_size = self.max_size().unwrap();
        Some((min_size + max_size + 1) / 2)
    }

    /// The middle size rounded down halfway between min and max byte size
    pub fn middle_byte_size(&self) -> Option<u16> {
        match self {
            DocumentPropertyType::Array(_) | DocumentPropertyType::VariableTypeArray(_) => {
                return None
            }
            _ => {}
        }
        let min_size = self.min_byte_size().unwrap();
        let max_size = self.max_byte_size().unwrap();
        Some((min_size + max_size) / 2)
    }

    /// The middle size rounded up halfway between min and max byte size
    pub fn middle_byte_size_ceil(&self) -> Option<u16> {
        match self {
            DocumentPropertyType::Array(_) | DocumentPropertyType::VariableTypeArray(_) => {
                return None
            }
            _ => {}
        }
        let min_size = self.min_byte_size().unwrap() as u32;
        let max_size = self.max_byte_size().unwrap() as u32;
        Some(((min_size + max_size + 1) / 2) as u16)
    }

    pub fn random_size(&self, rng: &mut StdRng) -> u16 {
        let min_size = self.min_size().unwrap();
        let max_size = self.max_size().unwrap();
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
            DocumentPropertyType::Identifier => Value::Identifier(rng.gen()),
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
            DocumentPropertyType::Identifier => Value::Identifier(rng.gen()),
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
            DocumentPropertyType::Identifier => Value::Identifier(rng.gen()),
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
                DataContractError::CorruptedSerialization(
                    "error reading varint from serialized document".to_string(),
                )
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
            DocumentPropertyType::Identifier => {
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
            DocumentPropertyType::Identifier => {
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
        return match self {
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
            DocumentPropertyType::Identifier => Ok(value.to_identifier_bytes()?),
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
        };
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
            DocumentPropertyType::Identifier => value
                .to_identifier_bytes()
                .map_err(ProtocolError::ValueError),
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
                if value == &vec![0] {
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
            DocumentPropertyType::Identifier => {
                let identifier = Identifier::from_bytes(value)?;
                Ok(identifier.into())
            }
            DocumentPropertyType::Boolean => {
                if value == &vec![0] {
                    Ok(Value::Bool(false))
                } else if value == &vec![1] {
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
            DocumentPropertyType::Identifier => Ok(Value::Identifier(
                Value::Text(str.to_owned())
                    .to_identifier()
                    .map_err(|e| DataContractError::ValueDecodingError(format!("{:?}", e)))?
                    .into_buffer(),
            )),
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

    /// Decodes an unsigned integer on 32 bits.
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
}

fn get_field_type_matching_error(value: &Value) -> DataContractError {
    DataContractError::ValueWrongType(format!(
        "document field type doesn't match \"{}\" document value",
        value
    ))
}
