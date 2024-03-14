use std::convert::TryInto;

use std::io::{BufReader, Read};

use crate::data_contract::errors::DataContractError;

use crate::consensus::basic::decode::DecodingError;
use crate::prelude::TimestampMillis;
use crate::ProtocolError;
use array::ArrayItemType;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use integer_encoding::{VarInt, VarIntReader};
use platform_value::Value;
use rand::distributions::{Alphanumeric, Standard};
use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub mod array;

// This struct will be changed in future to support more validation logic and serialization
// It will become versioned and it will be introduced by a new document type version
// @append_only
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct DocumentProperty {
    pub property_type: DocumentPropertyType,
    pub required: bool,
}

// @append_only
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum DocumentPropertyType {
    ///Todo decompose integer
    Integer,
    Number,
    String(Option<u16>, Option<u16>),    // TODO use structure
    ByteArray(Option<u16>, Option<u16>), // TODO user structure
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
            "integer" => Ok(DocumentPropertyType::Integer),
            "number" => Ok(DocumentPropertyType::Number),
            "boolean" => Ok(DocumentPropertyType::Boolean),
            "date" => Ok(DocumentPropertyType::Date),
            "identifier" => Ok(DocumentPropertyType::Identifier),
            _ => Err(DataContractError::ValueWrongType(
                "invalid type".to_string(),
            )),
        }
    }

    pub fn name(&self) -> String {
        match self {
            DocumentPropertyType::Integer => "integer".to_string(),
            DocumentPropertyType::Number => "number".to_string(),
            DocumentPropertyType::String(_, _) => "string".to_string(),
            DocumentPropertyType::ByteArray(_, _) => "byteArray".to_string(),
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
            DocumentPropertyType::Integer => Some(8),
            DocumentPropertyType::Number => Some(8),
            DocumentPropertyType::String(min_length, _) => match min_length {
                None => Some(0),
                Some(size) => Some(*size),
            },
            DocumentPropertyType::ByteArray(min_size, _) => match min_size {
                None => Some(0),
                Some(size) => Some(*size),
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
            DocumentPropertyType::Integer => Some(8),
            DocumentPropertyType::Number => Some(8),
            DocumentPropertyType::String(min_length, _) => match min_length {
                None => Some(0),
                Some(size) => Some(*size * 4),
            },
            DocumentPropertyType::ByteArray(min_size, _) => match min_size {
                None => Some(0),
                Some(size) => Some(*size),
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
            DocumentPropertyType::Integer => Some(8),
            DocumentPropertyType::Number => Some(8),
            DocumentPropertyType::String(_, max_length) => match max_length {
                None => Some(u16::MAX),
                Some(size) => Some(*size * 4),
            },
            DocumentPropertyType::ByteArray(_, max_size) => match max_size {
                None => Some(u16::MAX),
                Some(size) => Some(*size),
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
            DocumentPropertyType::Integer => Some(8),
            DocumentPropertyType::Number => Some(8),
            DocumentPropertyType::String(_, max_length) => match max_length {
                None => Some(16383),
                Some(size) => Some(*size),
            },
            DocumentPropertyType::ByteArray(_, max_size) => match max_size {
                None => Some(u16::MAX),
                Some(size) => Some(*size),
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
            DocumentPropertyType::Integer => Value::I64(rng.gen::<i64>()),
            DocumentPropertyType::Number => Value::Float(rng.gen::<f64>()),
            DocumentPropertyType::String(_, _) => {
                let size = self.random_size(rng);
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size as usize)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentPropertyType::ByteArray(_, _) => {
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
            DocumentPropertyType::Integer => Value::I64(rng.gen::<i64>()),
            DocumentPropertyType::Number => Value::Float(rng.gen::<f64>()),
            DocumentPropertyType::String(_, _) => {
                let size = self.min_size().unwrap();
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size as usize)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentPropertyType::ByteArray(_, _) => {
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
            DocumentPropertyType::Integer => Value::I64(rng.gen::<i64>()),
            DocumentPropertyType::Number => Value::Float(rng.gen::<f64>()),
            DocumentPropertyType::String(_, _) => {
                let size = self.max_size().unwrap();
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size as usize)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentPropertyType::ByteArray(_, _) => {
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
            DocumentPropertyType::String(_, _) => {
                let bytes = Self::read_varint_value(buf)?;
                let string = String::from_utf8(bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading string from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::Text(string)), false))
            }
            DocumentPropertyType::Date | DocumentPropertyType::Number => {
                let date = buf.read_f64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading date/number from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::Float(date)), false))
            }
            DocumentPropertyType::Integer => {
                let integer = buf.read_i64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading integer from serialized document".to_string(),
                    )
                })?;
                Ok((Some(Value::I64(integer)), false))
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
            DocumentPropertyType::ByteArray(min, max) => {
                match (min, max) {
                    (Some(min), Some(max)) if min == max => {
                        // if min == max, then we don't need a varint for the length
                        let len = *min as usize;
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
            DocumentPropertyType::String(_, _) => {
                if let Value::Text(value) = value {
                    let vec = value.into_bytes();
                    let mut r_vec = vec.len().encode_var_vec();
                    r_vec.extend(vec);
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error().into())
                }
            }
            DocumentPropertyType::Date => {
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
            DocumentPropertyType::Integer => {
                let value_as_i64: i64 = value.into_integer().map_err(ProtocolError::ValueError)?;
                let mut value_bytes = value_as_i64.to_be_bytes().to_vec();
                if required {
                    Ok(value_bytes)
                } else {
                    // if the value wasn't required we need to add a byte to prove it existed
                    let mut r_vec = vec![255u8];
                    r_vec.append(&mut value_bytes);
                    Ok(r_vec)
                }
            }
            DocumentPropertyType::Number => {
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
            DocumentPropertyType::ByteArray(_, _) => {
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
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
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
                    Err(get_field_type_matching_error().into())
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
                    Err(get_field_type_matching_error().into())
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
            DocumentPropertyType::String(_, _) => {
                let value_as_text = value.as_text().ok_or_else(get_field_type_matching_error)?;
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
            DocumentPropertyType::Integer => {
                let value_as_i64: i64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                Ok(value_as_i64.to_be_bytes().to_vec())
            }
            DocumentPropertyType::Number => {
                let value_as_f64 = value.to_float().map_err(ProtocolError::ValueError)?;
                Ok(value_as_f64.to_be_bytes().to_vec())
            }
            DocumentPropertyType::ByteArray(min, max) => match (min, max) {
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
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
                // 0 means does not exist
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![0]) // 0 is false
                }
            }
            DocumentPropertyType::Object(inner_fields) => {
                let Some(value_map) = value.as_map() else {
                    return Err(get_field_type_matching_error().into());
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
                    Err(get_field_type_matching_error().into())
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
            DocumentPropertyType::String(_, _) => {
                let value_as_text = value.as_text().ok_or_else(get_field_type_matching_error)?;
                let vec = value_as_text.as_bytes().to_vec();
                if vec.is_empty() {
                    // we don't want to collide with the definition of an empty string
                    Ok(vec![0])
                } else {
                    Ok(vec)
                }
            }
            DocumentPropertyType::Date => DocumentPropertyType::encode_date_timestamp(
                value.to_integer().map_err(ProtocolError::ValueError)?,
            ),
            DocumentPropertyType::Integer => {
                let value_as_i64 = value.to_integer().map_err(ProtocolError::ValueError)?;

                DocumentPropertyType::encode_signed_integer(value_as_i64)
            }
            DocumentPropertyType::Number => Ok(Self::encode_float(
                value.to_float().map_err(ProtocolError::ValueError)?,
            )),
            DocumentPropertyType::ByteArray(_, _) => {
                value.to_binary_bytes().map_err(ProtocolError::ValueError)
            }
            DocumentPropertyType::Identifier => value
                .to_identifier_bytes()
                .map_err(ProtocolError::ValueError),
            DocumentPropertyType::Boolean => {
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
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

    // Given a field type and a value this function chooses and executes the right encoding method
    pub fn value_from_string(&self, str: &str) -> Result<Value, DataContractError> {
        match self {
            DocumentPropertyType::String(min, max) => {
                if let Some(min) = min {
                    if str.len() < *min as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "string is too small".to_string(),
                        ));
                    }
                }
                if let Some(max) = max {
                    if str.len() > *max as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "string is too big".to_string(),
                        ));
                    }
                }
                Ok(Value::Text(str.to_string()))
            }
            DocumentPropertyType::Integer => str.parse::<i128>().map(Value::I128).map_err(|_| {
                DataContractError::ValueWrongType("value is not an integer from string".to_string())
            }),
            DocumentPropertyType::Number | DocumentPropertyType::Date => {
                str.parse::<f64>().map(Value::Float).map_err(|_| {
                    DataContractError::ValueWrongType(
                        "value is not a float from string".to_string(),
                    )
                })
            }
            DocumentPropertyType::ByteArray(min, max) => {
                if let Some(min) = min {
                    if str.len() / 2 < *min as usize {
                        return Err(DataContractError::FieldRequirementUnmet(
                            "byte array is too small".to_string(),
                        ));
                    }
                }
                if let Some(max) = max {
                    if str.len() / 2 > *max as usize {
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

    pub fn encode_date_timestamp(val: TimestampMillis) -> Result<Vec<u8>, ProtocolError> {
        Self::encode_unsigned_integer(val)
    }

    pub fn encode_unsigned_integer(val: u64) -> Result<Vec<u8>, ProtocolError> {
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

        Ok(wtr)
    }

    pub fn encode_signed_integer(val: i64) -> Result<Vec<u8>, ProtocolError> {
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

        Ok(wtr)
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
}

fn get_field_type_matching_error() -> DataContractError {
    DataContractError::ValueWrongType(
        "document field type doesn't match document value".to_string(),
    )
}
