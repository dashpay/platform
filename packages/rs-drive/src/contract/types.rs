use std::collections::BTreeMap;
use std::fmt;
use std::io::{BufReader, Read};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use ciborium::value::{Integer, Value};
use integer_encoding::{VarInt, VarIntReader};
use rand::distributions::{Alphanumeric, Standard};
use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::common::{cbor_map_to_btree_map, cbor_owned_map_to_btree_map};
use crate::error::contract::ContractError;
use crate::error::drive::DriveError;
use crate::error::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct DocumentField {
    pub document_type: DocumentFieldType,
    pub required: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ArrayFieldType {
    Integer,
    Number,
    String(Option<usize>, Option<usize>),
    ByteArray(Option<usize>, Option<usize>),
    Boolean,
    Date,
}

impl ArrayFieldType {
    pub fn encode_value_with_size(&self, value: Value) -> Result<Vec<u8>, Error> {
        return match self {
            ArrayFieldType::String(_, _) => {
                if let Value::Text(value) = value {
                    let vec = value.into_bytes();
                    let mut r_vec = vec.len().encode_var_vec();
                    r_vec.extend(vec);
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error())
                }
            }
            ArrayFieldType::Date => {
                let value_as_f64 = match value {
                    Value::Integer(value_as_integer) => {
                        let value_as_i128: i128 = value_as_integer.try_into().map_err(|_| {
                            Error::Contract(ContractError::ValueWrongType("expected integer value"))
                        })?;
                        let value_as_f64: f64 = value_as_i128 as f64;
                        Ok(value_as_f64)
                    }
                    Value::Float(value_as_float) => Ok(value_as_float),
                    _ => Err(get_field_type_matching_error()),
                }?;
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayFieldType::Integer => {
                let value_as_integer = value
                    .as_integer()
                    .ok_or_else(get_field_type_matching_error)?;

                let value_as_i64: i64 = value_as_integer.try_into().map_err(|_| {
                    Error::Contract(ContractError::ValueWrongType("expected integer value"))
                })?;
                let value_bytes = value_as_i64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayFieldType::Number => {
                let value_as_f64 = if value.is_integer() {
                    let value_as_integer = value
                        .as_integer()
                        .ok_or_else(get_field_type_matching_error)?;

                    let value_as_i64: i64 = value_as_integer.try_into().map_err(|_| {
                        Error::Contract(ContractError::ValueWrongType("expected number value"))
                    })?;

                    value_as_i64 as f64
                } else {
                    value.as_float().ok_or_else(get_field_type_matching_error)?
                };
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayFieldType::ByteArray(_, _) => {
                let mut bytes = match value {
                    Value::Bytes(bytes) => Ok(bytes),
                    Value::Text(text) => {
                        let value_as_bytes = base64::decode(text).map_err(|_| {
                            Error::Contract(ContractError::ValueDecodingError(
                                "bytearray: invalid base64 value",
                            ))
                        })?;
                        Ok(value_as_bytes)
                    }
                    Value::Array(array) => array
                        .into_iter()
                        .map(|byte| match byte {
                            Value::Integer(int) => {
                                let value_as_u8: u8 = int.try_into().map_err(|_| {
                                    Error::Contract(ContractError::ValueWrongType(
                                        "expected u8 value",
                                    ))
                                })?;
                                Ok(value_as_u8)
                            }
                            _ => Err(Error::Contract(ContractError::ValueWrongType(
                                "not an array of integers",
                            ))),
                        })
                        .collect::<Result<Vec<u8>, Error>>(),
                    _ => Err(get_field_type_matching_error()),
                }?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            ArrayFieldType::Boolean => {
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![0]) // 2 is false
                }
            }
        };
    }

    pub fn encode_value_ref_with_size(&self, value: &Value) -> Result<Vec<u8>, Error> {
        return match self {
            ArrayFieldType::String(_, _) => {
                let value_as_text = value.as_text().ok_or_else(get_field_type_matching_error)?;
                let vec = value_as_text.as_bytes().to_vec();
                let mut r_vec = vec.len().encode_var_vec();
                r_vec.extend(vec);
                Ok(r_vec)
            }
            ArrayFieldType::Date => {
                let value_as_f64 = match *value {
                    Value::Integer(value_as_integer) => {
                        let value_as_i128: i128 = value_as_integer.try_into().map_err(|_| {
                            Error::Contract(ContractError::ValueWrongType("expected integer value"))
                        })?;
                        let value_as_f64: f64 = value_as_i128 as f64;
                        Ok(value_as_f64)
                    }
                    Value::Float(value_as_float) => Ok(value_as_float),
                    _ => Err(get_field_type_matching_error()),
                }?;
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayFieldType::Integer => {
                let value_as_integer = value
                    .as_integer()
                    .ok_or_else(get_field_type_matching_error)?;

                let value_as_i64: i64 = value_as_integer.try_into().map_err(|_| {
                    Error::Contract(ContractError::ValueWrongType("expected integer value"))
                })?;
                let value_bytes = value_as_i64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayFieldType::Number => {
                let value_as_f64 = if value.is_integer() {
                    let value_as_integer = value
                        .as_integer()
                        .ok_or_else(get_field_type_matching_error)?;

                    let value_as_i64: i64 = value_as_integer.try_into().map_err(|_| {
                        Error::Contract(ContractError::ValueWrongType("expected number value"))
                    })?;

                    value_as_i64 as f64
                } else {
                    value.as_float().ok_or_else(get_field_type_matching_error)?
                };
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayFieldType::ByteArray(_, _) => {
                let mut bytes = match value {
                    Value::Bytes(bytes) => Ok(bytes.clone()),
                    Value::Text(text) => {
                        let value_as_bytes = base64::decode(text).map_err(|_| {
                            Error::Contract(ContractError::ValueDecodingError(
                                "bytearray: invalid base64 value",
                            ))
                        })?;
                        Ok(value_as_bytes)
                    }
                    Value::Array(array) => array
                        .iter()
                        .map(|byte| match byte {
                            Value::Integer(int) => {
                                let value_as_u8: u8 = (*int).try_into().map_err(|_| {
                                    Error::Contract(ContractError::ValueWrongType(
                                        "expected u8 value",
                                    ))
                                })?;
                                Ok(value_as_u8)
                            }
                            _ => Err(Error::Contract(ContractError::ValueWrongType(
                                "not an array of integers",
                            ))),
                        })
                        .collect::<Result<Vec<u8>, Error>>(),
                    _ => Err(get_field_type_matching_error()),
                }?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            ArrayFieldType::Boolean => {
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
                // 0 means does not exist
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![0]) // 2 is false
                }
            }
        };
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum DocumentFieldType {
    Integer,
    Number,
    String(Option<usize>, Option<usize>),
    ByteArray(Option<usize>, Option<usize>),
    Boolean,
    Date,
    Object(BTreeMap<String, DocumentField>),
    Array(ArrayFieldType),
    VariableTypeArray(Vec<ArrayFieldType>),
}

impl DocumentFieldType {
    pub fn min_size(&self) -> Option<usize> {
        match self {
            DocumentFieldType::Integer => Some(8),
            DocumentFieldType::Number => Some(8),
            DocumentFieldType::String(min_length, _) => match min_length {
                None => Some(0),
                Some(size) => Some(*size),
            },
            DocumentFieldType::ByteArray(min_size, _) => match min_size {
                None => Some(0),
                Some(size) => Some(*size),
            },
            DocumentFieldType::Boolean => Some(1),
            DocumentFieldType::Date => Some(8),
            DocumentFieldType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.document_type.min_size())
                .sum(),
            DocumentFieldType::Array(_) => None,
            DocumentFieldType::VariableTypeArray(_) => None,
        }
    }

    pub fn min_byte_size(&self) -> Option<usize> {
        match self {
            DocumentFieldType::Integer => Some(8),
            DocumentFieldType::Number => Some(8),
            DocumentFieldType::String(min_length, _) => match min_length {
                None => Some(0),
                Some(size) => Some(*size * 4),
            },
            DocumentFieldType::ByteArray(min_size, _) => match min_size {
                None => Some(0),
                Some(size) => Some(*size),
            },
            DocumentFieldType::Boolean => Some(1),
            DocumentFieldType::Date => Some(8),
            DocumentFieldType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.document_type.min_byte_size())
                .sum(),
            DocumentFieldType::Array(_) => None,
            DocumentFieldType::VariableTypeArray(_) => None,
        }
    }

    pub fn max_byte_size(&self) -> Option<usize> {
        match self {
            DocumentFieldType::Integer => Some(8),
            DocumentFieldType::Number => Some(8),
            DocumentFieldType::String(_, max_length) => match max_length {
                None => Some(16384),
                Some(size) => Some(*size * 4),
            },
            DocumentFieldType::ByteArray(_, max_size) => match max_size {
                None => Some(65536),
                Some(size) => Some(*size),
            },
            DocumentFieldType::Boolean => Some(1),
            DocumentFieldType::Date => Some(8),
            DocumentFieldType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.document_type.max_byte_size())
                .sum(),
            DocumentFieldType::Array(_) => None,
            DocumentFieldType::VariableTypeArray(_) => None,
        }
    }

    pub fn max_size(&self) -> Option<usize> {
        match self {
            DocumentFieldType::Integer => Some(8),
            DocumentFieldType::Number => Some(8),
            DocumentFieldType::String(_, max_length) => match max_length {
                None => Some(16384),
                Some(size) => Some(*size),
            },
            DocumentFieldType::ByteArray(_, max_size) => match max_size {
                None => Some(65536),
                Some(size) => Some(*size),
            },
            DocumentFieldType::Boolean => Some(1),
            DocumentFieldType::Date => Some(8),
            DocumentFieldType::Object(sub_fields) => sub_fields
                .iter()
                .map(|(_, sub_field)| sub_field.document_type.max_size())
                .sum(),
            DocumentFieldType::Array(_) => None,
            DocumentFieldType::VariableTypeArray(_) => None,
        }
    }

    pub fn random_size(&self, rng: &mut StdRng) -> usize {
        let min_size = self.min_size().unwrap();
        let max_size = self.max_size().unwrap();
        rng.gen_range(min_size..=max_size)
    }

    pub fn random_value(&self, rng: &mut StdRng) -> Value {
        match self {
            DocumentFieldType::Integer => {
                Value::Integer(Integer::try_from(rng.gen::<i64>()).unwrap())
            }
            DocumentFieldType::Number => Value::Float(rng.gen::<f64>()),
            DocumentFieldType::String(_, _) => {
                let size = self.random_size(rng);
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentFieldType::ByteArray(_, _) => {
                let size = self.random_size(rng);
                Value::Bytes(rng.sample_iter(Standard).take(size).collect())
            }
            DocumentFieldType::Boolean => Value::Bool(rng.gen::<bool>()),
            DocumentFieldType::Date => {
                let f: f64 = rng.gen_range(1548910575000.0..1648910575000.0);
                Value::Float(f.round() / 1000.0)
            }
            DocumentFieldType::Object(sub_fields) => {
                let value_vec = sub_fields
                    .iter()
                    .map(|(string, field_type)| {
                        (
                            Value::Text(string.clone()),
                            field_type.document_type.random_value(rng),
                        )
                    })
                    .collect();
                Value::Map(value_vec)
            }
            DocumentFieldType::Array(_) => Value::Null,
            DocumentFieldType::VariableTypeArray(_) => Value::Null,
        }
    }

    pub fn random_filled_value(&self, rng: &mut StdRng) -> Value {
        match self {
            DocumentFieldType::Integer => {
                Value::Integer(Integer::try_from(rng.gen::<i64>()).unwrap())
            }
            DocumentFieldType::Number => Value::Float(rng.gen::<f64>()),
            DocumentFieldType::String(_, _) => {
                let size = self.max_size().unwrap();
                Value::Text(
                    rng.sample_iter(Alphanumeric)
                        .take(size)
                        .map(char::from)
                        .collect(),
                )
            }
            DocumentFieldType::ByteArray(_, _) => {
                let size = self.max_size().unwrap();
                Value::Bytes(rng.sample_iter(Standard).take(size).collect())
            }
            DocumentFieldType::Boolean => Value::Bool(rng.gen::<bool>()),
            DocumentFieldType::Date => {
                let f: f64 = rng.gen_range(1548910575000.0..1648910575000.0);
                Value::Float(f.round() / 1000.0)
            }
            DocumentFieldType::Object(sub_fields) => {
                let value_vec = sub_fields
                    .iter()
                    .map(|(string, field_type)| {
                        (
                            Value::Text(string.clone()),
                            field_type.document_type.random_filled_value(rng),
                        )
                    })
                    .collect();
                Value::Map(value_vec)
            }
            DocumentFieldType::Array(_) => Value::Null,
            DocumentFieldType::VariableTypeArray(_) => Value::Null,
        }
    }

    fn read_varint_value(buf: &mut BufReader<&[u8]>) -> Result<Option<Vec<u8>>, Error> {
        let bytes: usize = buf.read_varint().map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })?;
        if bytes == 0 {
            Ok(None)
        } else {
            let mut value: Vec<u8> = vec![0u8; bytes];
            buf.read_exact(&mut value).map_err(|_| {
                Error::Drive(DriveError::CorruptedSerialization(
                    "error reading from serialized document",
                ))
            })?;
            Ok(Some(value))
        }
    }

    pub fn read_from(
        &self,
        buf: &mut BufReader<&[u8]>,
        required: bool,
    ) -> Result<Option<Value>, Error> {
        return match self {
            DocumentFieldType::String(_, _) => {
                let bytes = Self::read_varint_value(buf)?;
                if let Some(bytes) = bytes {
                    let string = String::from_utf8(bytes).map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "error reading from serialized document",
                        ))
                    })?;
                    Ok(Some(Value::Text(string)))
                } else {
                    Ok(None)
                }
            }
            DocumentFieldType::Date | DocumentFieldType::Number => {
                if required == false {
                    let marker = buf.read_u8().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "error reading from serialized document",
                        ))
                    })?;
                    if marker == 0 {
                        return Ok(None);
                    }
                }
                let date = buf.read_f64::<BigEndian>().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "error reading from serialized document",
                    ))
                })?;
                Ok(Some(Value::Float(date)))
            }
            DocumentFieldType::Integer => {
                if required == false {
                    let marker = buf.read_u8().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "error reading from serialized document",
                        ))
                    })?;
                    if marker == 0 {
                        return Ok(None);
                    }
                }
                let integer = buf.read_i64::<BigEndian>().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "error reading from serialized document",
                    ))
                })?;
                Ok(Some(Value::Integer(Integer::from(integer))))
            }
            DocumentFieldType::Boolean => {
                let value = buf.read_u8().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "error reading from serialized document",
                    ))
                })?;
                match value {
                    0 => Ok(None),
                    1 => Ok(Some(Value::Bool(true))),
                    _ => Ok(Some(Value::Bool(false))),
                }
            }
            DocumentFieldType::ByteArray(_, _) => {
                let bytes = Self::read_varint_value(buf)?;
                Ok(bytes.map(Value::Bytes))
            }

            DocumentFieldType::Object(inner_fields) => {
                let values = inner_fields
                    .iter()
                    .filter_map(|(key, field)| {
                        let read_value = field.document_type.read_from(buf, field.required);
                        match read_value {
                            Ok(read_value) => {
                                if let Some(read_value) = read_value {
                                    Some(Ok((Value::Text(key.clone()), read_value)))
                                } else {
                                    None
                                }
                            }
                            Err(e) => Some(Err(e)),
                        }
                    })
                    .collect::<Result<Vec<(Value, Value)>, Error>>()?;
                if values.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(Value::Map(values)))
                }
            }
            DocumentFieldType::Array(array_field_type) => {
                Err(Error::Drive(DriveError::Unsupported(
                    "serialization of arrays not yet supported",
                )))
                // cbor_inner_array_value(value.as_array().ok_or_else(get_field_type_matching_error))
                // let array = value.as_array().ok_or_else(get_field_type_matching_error)?;
            }
            DocumentFieldType::VariableTypeArray(_) => Err(Error::Drive(DriveError::Unsupported(
                "serialization of arrays not yet supported",
            ))),
        };
    }

    pub fn encode_value_with_size(&self, value: Value, required: bool) -> Result<Vec<u8>, Error> {
        if value.is_null() {
            return Ok(vec![]);
        }
        return match self {
            DocumentFieldType::String(_, _) => {
                if let Value::Text(value) = value {
                    let vec = value.into_bytes();
                    let mut r_vec = vec.len().encode_var_vec();
                    r_vec.extend(vec);
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error())
                }
            }
            DocumentFieldType::Date => {
                let value_as_f64 = match value {
                    Value::Integer(value_as_integer) => {
                        let value_as_i128: i128 = value_as_integer.try_into().map_err(|_| {
                            Error::Contract(ContractError::ValueWrongType("expected integer value"))
                        })?;
                        let value_as_f64: f64 = value_as_i128 as f64;
                        Ok(value_as_f64)
                    }
                    Value::Float(value_as_float) => Ok(value_as_float),
                    _ => Err(get_field_type_matching_error()),
                }?;
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
            DocumentFieldType::Integer => {
                let value_as_integer = value
                    .as_integer()
                    .ok_or_else(get_field_type_matching_error)?;

                let value_as_i64: i64 = value_as_integer.try_into().map_err(|_| {
                    Error::Contract(ContractError::ValueWrongType("expected integer value"))
                })?;
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
            DocumentFieldType::Number => {
                let value_as_f64 = if value.is_integer() {
                    let value_as_integer = value
                        .as_integer()
                        .ok_or_else(get_field_type_matching_error)?;

                    let value_as_i64: i64 = value_as_integer.try_into().map_err(|_| {
                        Error::Contract(ContractError::ValueWrongType("expected number value"))
                    })?;

                    value_as_i64 as f64
                } else {
                    value.as_float().ok_or_else(get_field_type_matching_error)?
                };
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
            DocumentFieldType::ByteArray(_, _) => {
                let mut bytes = match value {
                    Value::Bytes(bytes) => Ok(bytes),
                    Value::Text(text) => {
                        let value_as_bytes = base64::decode(text).map_err(|_| {
                            Error::Contract(ContractError::ValueDecodingError(
                                "bytearray: invalid base64 value",
                            ))
                        })?;
                        Ok(value_as_bytes)
                    }
                    Value::Array(array) => array
                        .into_iter()
                        .map(|byte| match byte {
                            Value::Integer(int) => {
                                let value_as_u8: u8 = int.try_into().map_err(|_| {
                                    Error::Contract(ContractError::ValueWrongType(
                                        "expected u8 value",
                                    ))
                                })?;
                                Ok(value_as_u8)
                            }
                            _ => Err(Error::Contract(ContractError::ValueWrongType(
                                "not an array of integers",
                            ))),
                        })
                        .collect::<Result<Vec<u8>, Error>>(),
                    _ => Err(get_field_type_matching_error()),
                }?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            DocumentFieldType::Boolean => {
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
                // 0 means does not exist
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![2]) // 2 is false
                }
            }
            DocumentFieldType::Object(inner_fields) => {
                if let Value::Map(map) = value {
                    let mut value_map = cbor_owned_map_to_btree_map(map);
                    let mut r_vec = vec![];
                    inner_fields
                        .into_iter()
                        .map(|(key, field)| {
                            if let Some(value) = value_map.remove(key) {
                                let mut serialized_value = field
                                    .document_type
                                    .encode_value_with_size(value, field.required)?;
                                r_vec.append(&mut serialized_value);
                                Ok(())
                            } else if field.required {
                                Err(Error::Contract(ContractError::MissingRequiredKey(
                                    "a required field is not present",
                                )))
                            } else {
                                // We don't have something that wasn't required
                                r_vec.push(0);
                                Ok(())
                            }
                        })
                        .collect::<Result<(), Error>>()?;
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error())
                }
            }
            DocumentFieldType::Array(array_field_type) => {
                if let Value::Array(array) = value {
                    let mut r_vec = array.len().encode_var_vec();

                    array
                        .into_iter()
                        .map(|value| {
                            let mut serialized_value =
                                array_field_type.encode_value_with_size(value)?;
                            r_vec.append(&mut serialized_value);
                            Ok(())
                        })
                        .collect::<Result<(), Error>>()?;
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error())
                }
            }
            DocumentFieldType::VariableTypeArray(_) => Err(Error::Drive(DriveError::Unsupported(
                "serialization of variable type arrays not yet supported",
            ))),
        };
    }

    pub fn encode_value_ref_with_size(
        &self,
        value: &Value,
        required: bool,
    ) -> Result<Vec<u8>, Error> {
        if value.is_null() {
            return Ok(vec![]);
        }
        return match self {
            DocumentFieldType::String(_, _) => {
                let value_as_text = value.as_text().ok_or_else(get_field_type_matching_error)?;
                let vec = value_as_text.as_bytes().to_vec();
                let mut r_vec = vec.len().encode_var_vec();
                r_vec.extend(vec);
                Ok(r_vec)
            }
            DocumentFieldType::Date => {
                let value_as_f64 = match *value {
                    Value::Integer(value_as_integer) => {
                        let value_as_i128: i128 = value_as_integer.try_into().map_err(|_| {
                            Error::Contract(ContractError::ValueWrongType("expected integer value"))
                        })?;
                        let value_as_f64: f64 = value_as_i128 as f64;
                        Ok(value_as_f64)
                    }
                    Value::Float(value_as_float) => Ok(value_as_float),
                    _ => Err(get_field_type_matching_error()),
                }?;
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
            DocumentFieldType::Integer => {
                let value_as_integer = value
                    .as_integer()
                    .ok_or_else(get_field_type_matching_error)?;

                let value_as_i64: i64 = value_as_integer.try_into().map_err(|_| {
                    Error::Contract(ContractError::ValueWrongType("expected integer value"))
                })?;
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
            DocumentFieldType::Number => {
                let value_as_f64 = if value.is_integer() {
                    let value_as_integer = value
                        .as_integer()
                        .ok_or_else(get_field_type_matching_error)?;

                    let value_as_i64: i64 = value_as_integer.try_into().map_err(|_| {
                        Error::Contract(ContractError::ValueWrongType("expected number value"))
                    })?;

                    value_as_i64 as f64
                } else {
                    value.as_float().ok_or_else(get_field_type_matching_error)?
                };
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
            DocumentFieldType::ByteArray(_, _) => {
                let mut bytes = match value {
                    Value::Bytes(bytes) => Ok(bytes.clone()),
                    Value::Text(text) => {
                        let value_as_bytes = base64::decode(text).map_err(|_| {
                            Error::Contract(ContractError::ValueDecodingError(
                                "bytearray: invalid base64 value",
                            ))
                        })?;
                        Ok(value_as_bytes)
                    }
                    Value::Array(array) => array
                        .iter()
                        .map(|byte| match byte {
                            Value::Integer(int) => {
                                let value_as_u8: u8 = (*int).try_into().map_err(|_| {
                                    Error::Contract(ContractError::ValueWrongType(
                                        "expected u8 value",
                                    ))
                                })?;
                                Ok(value_as_u8)
                            }
                            _ => Err(Error::Contract(ContractError::ValueWrongType(
                                "not an array of integers",
                            ))),
                        })
                        .collect::<Result<Vec<u8>, Error>>(),
                    _ => Err(get_field_type_matching_error()),
                }?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            DocumentFieldType::Boolean => {
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
                // 0 means does not exist
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![2]) // 2 is false
                }
            }
            DocumentFieldType::Object(inner_fields) => {
                let value_map = cbor_map_to_btree_map(
                    value.as_map().ok_or_else(get_field_type_matching_error)?,
                );
                let mut r_vec = vec![];
                inner_fields
                    .iter()
                    .map(|(key, field)| {
                        if let Some(value) = value_map.get(key) {
                            let value = field
                                .document_type
                                .encode_value_ref_with_size(value, field.required)?;
                            r_vec.extend(value.as_slice());
                            Ok(())
                        } else if field.required {
                            Err(Error::Contract(ContractError::MissingRequiredKey(
                                "a required field is not present",
                            )))
                        } else {
                            // We don't have something that wasn't required
                            r_vec.push(0);
                            Ok(())
                        }
                    })
                    .collect::<Result<(), Error>>()?;
                Ok(r_vec)
            }
            DocumentFieldType::Array(array_field_type) => {
                if let Value::Array(array) = value {
                    let mut r_vec = array.len().encode_var_vec();

                    array
                        .into_iter()
                        .map(|value| {
                            let mut serialized_value =
                                array_field_type.encode_value_ref_with_size(value)?;
                            r_vec.append(&mut serialized_value);
                            Ok(())
                        })
                        .collect::<Result<(), Error>>()?;
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error())
                }
            }
            DocumentFieldType::VariableTypeArray(_) => Err(Error::Drive(DriveError::Unsupported(
                "serialization of arrays not yet supported",
            ))),
        };
    }

    // Given a field type and a value this function chooses and executes the right encoding method
    pub fn encode_value_for_tree_keys(&self, value: &Value) -> Result<Vec<u8>, Error> {
        if value.is_null() {
            return Ok(vec![]);
        }
        return match self {
            DocumentFieldType::String(_, _) => {
                let value_as_text = value.as_text().ok_or_else(get_field_type_matching_error)?;
                let vec = value_as_text.as_bytes().to_vec();
                if vec.is_empty() {
                    // we don't want to collide with the definition of an empty string
                    Ok(vec![0])
                } else {
                    Ok(vec)
                }
            }
            DocumentFieldType::Date => match *value {
                Value::Integer(value_as_integer) => {
                    let value_as_i128: i128 = value_as_integer.try_into().map_err(|_| {
                        Error::Contract(ContractError::ValueWrongType("expected integer value"))
                    })?;
                    let value_as_f64: f64 = value_as_i128 as f64;

                    encode_float(value_as_f64)
                }
                Value::Float(value_as_float) => encode_float(value_as_float),
                _ => Err(get_field_type_matching_error()),
            },
            DocumentFieldType::Integer => {
                let value_as_integer = value
                    .as_integer()
                    .ok_or_else(get_field_type_matching_error)?;

                let value_as_i64: i64 = value_as_integer.try_into().map_err(|_| {
                    Error::Contract(ContractError::ValueWrongType("expected integer value"))
                })?;

                encode_signed_integer(value_as_i64)
            }
            DocumentFieldType::Number => {
                let value_as_f64 = if value.is_integer() {
                    let value_as_integer = value
                        .as_integer()
                        .ok_or_else(get_field_type_matching_error)?;

                    let value_as_i64: i64 = value_as_integer.try_into().map_err(|_| {
                        Error::Contract(ContractError::ValueWrongType("expected number value"))
                    })?;

                    value_as_i64 as f64
                } else {
                    value.as_float().ok_or_else(get_field_type_matching_error)?
                };

                encode_float(value_as_f64)
            }
            DocumentFieldType::ByteArray(_, _) => match value {
                Value::Bytes(bytes) => Ok(bytes.clone()),
                Value::Text(text) => {
                    let value_as_bytes = base64::decode(text).map_err(|_| {
                        Error::Contract(ContractError::ValueDecodingError(
                            "bytearray: invalid base64 value",
                        ))
                    })?;
                    Ok(value_as_bytes)
                }
                Value::Array(array) => array
                    .iter()
                    .map(|byte| match byte {
                        Value::Integer(int) => {
                            let value_as_u8: u8 = (*int).try_into().map_err(|_| {
                                Error::Contract(ContractError::ValueWrongType("expected u8 value"))
                            })?;
                            Ok(value_as_u8)
                        }
                        _ => Err(Error::Contract(ContractError::ValueWrongType(
                            "not an array of integers",
                        ))),
                    })
                    .collect::<Result<Vec<u8>, Error>>(),
                _ => Err(get_field_type_matching_error()),
            },
            DocumentFieldType::Boolean => {
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
                if value_as_boolean {
                    Ok(vec![1])
                } else {
                    Ok(vec![0])
                }
            }
            DocumentFieldType::Object(_) => Err(Error::Contract(
                ContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an object",
                ),
            )),
            DocumentFieldType::Array(_) | DocumentFieldType::VariableTypeArray(_) => Err(
                Error::Contract(ContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an array",
                )),
            ),
        };
    }

    // Given a field type and a value this function chooses and executes the right encoding method
    pub fn value_from_string(&self, str: &str) -> Result<Value, Error> {
        return match self {
            DocumentFieldType::String(min, max) => {
                if let Some(min) = min {
                    if str.len() < *min {
                        return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                            "string is too small",
                        )));
                    }
                }
                if let Some(max) = max {
                    if str.len() > *max {
                        return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                            "string is too big",
                        )));
                    }
                }
                Ok(Value::Text(str.to_string()))
            }
            DocumentFieldType::Integer => str
                .parse::<i128>()
                .map(|f| Value::Integer(Integer::try_from(f).unwrap()))
                .map_err(|_| {
                    Error::Contract(ContractError::ValueWrongType("value is not an integer"))
                }),
            DocumentFieldType::Number | DocumentFieldType::Date => {
                str.parse::<f64>().map(Value::Float).map_err(|_| {
                    Error::Contract(ContractError::ValueWrongType("value is not a float"))
                })
            }
            DocumentFieldType::ByteArray(min, max) => {
                if let Some(min) = min {
                    if str.len() / 2 < *min {
                        return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                            "byte array is too small",
                        )));
                    }
                }
                if let Some(max) = max {
                    if str.len() / 2 > *max {
                        return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                            "byte array  is too big",
                        )));
                    }
                }
                Ok(Value::Bytes(hex::decode(str).map_err(|_| {
                    Error::Contract(ContractError::ValueDecodingError(
                        "could not parse hex bytes",
                    ))
                })?))
            }
            DocumentFieldType::Boolean => {
                if str.to_lowercase().as_str() == "true" {
                    Ok(Value::Bool(true))
                } else if str.to_lowercase().as_str() == "false" {
                    Ok(Value::Bool(false))
                } else {
                    Err(Error::Contract(ContractError::ValueDecodingError(
                        "could not parse a boolean to a value",
                    )))
                }
            }
            DocumentFieldType::Object(_) => Err(Error::Contract(
                ContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an object",
                ),
            )),
            DocumentFieldType::Array(_) | DocumentFieldType::VariableTypeArray(_) => Err(
                Error::Contract(ContractError::EncodingDataStructureNotSupported(
                    "we should never try encoding an array",
                )),
            ),
        };
    }
}

impl fmt::Display for DocumentFieldType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = match self {
            DocumentFieldType::Integer => "integer".to_string(),
            DocumentFieldType::Number => "number".to_string(),
            DocumentFieldType::String(min, max) => {
                let min_string = if let Some(min) = min {
                    format!("min: {}", *min)
                } else {
                    "no min".to_string()
                };
                let max_string = if let Some(max) = max {
                    format!("max: {}", *max)
                } else {
                    "no max".to_string()
                };
                format!("string ({} / {})", min_string.as_str(), max_string.as_str())
            }
            DocumentFieldType::ByteArray(min, max) => {
                let min_bytes = if let Some(min) = min {
                    format!("min: {}", *min)
                } else {
                    "no min".to_string()
                };
                let max_bytes = if let Some(max) = max {
                    format!("max: {}", *max)
                } else {
                    "no max".to_string()
                };
                format!("bytes ({} / {})", min_bytes.as_str(), max_bytes.as_str())
            }
            DocumentFieldType::Boolean => "bool".to_string(),
            DocumentFieldType::Date => "date".to_string(),
            DocumentFieldType::Object(sub_fields) => {
                let object_rep = sub_fields
                    .iter()
                    .map(|(string, document_field)| {
                        format!(
                            "{} : {}{}",
                            string,
                            if document_field.required {
                                "required : "
                            } else {
                                ""
                            },
                            document_field.document_type
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(" | ");
                format!("object: {{ {} }}", object_rep)
            }
            DocumentFieldType::Array(_) | DocumentFieldType::VariableTypeArray(_) => {
                "array".to_string()
            }
        };
        write!(f, "{}", text.as_str())
    }
}

pub fn string_to_field_type(field_type_name: &str) -> Option<DocumentFieldType> {
    match field_type_name {
        "integer" => Some(DocumentFieldType::Integer),
        "number" => Some(DocumentFieldType::Number),
        "boolean" => Some(DocumentFieldType::Boolean),
        "date" => Some(DocumentFieldType::Date),
        _ => None,
    }
}

fn get_field_type_matching_error() -> Error {
    Error::Contract(ContractError::ValueWrongType(
        "document field type doesn't match document value",
    ))
}

pub fn encode_unsigned_integer(val: u64) -> Result<Vec<u8>, Error> {
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

pub fn encode_signed_integer(val: i64) -> Result<Vec<u8>, Error> {
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

pub fn encode_float(val: f64) -> Result<Vec<u8>, Error> {
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

    Ok(wtr)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ciborium::value::{Integer, Value};

    use crate::contract::types::DocumentFieldType;

    #[test]
    fn test_successful_encode() {
        // Constraint: for all types, if a > b then encoding(a) > encoding(b)
        let encode_err_msg = "should encode: valid parameters";

        // Integer encoding
        // Test approach
        // Test positive domain
        // Test negative domain
        // Test against 0
        // Test relationship between positive and negative domain

        // Show that the domain of positive integers maintains sort order after encoding
        let integer1 = Value::Integer(Integer::from(1));
        let integer2 = Value::Integer(Integer::from(600));
        let integer3 = Value::Integer(Integer::from(i64::MAX));

        let encoded_integer1 = &DocumentFieldType::Integer
            .encode_value_for_tree_keys(&integer1)
            .expect(encode_err_msg);
        let encoded_integer2 = &DocumentFieldType::Integer
            .encode_value_for_tree_keys(&integer2)
            .expect(encode_err_msg);
        let encoded_integer3 = &DocumentFieldType::Integer
            .encode_value_for_tree_keys(&integer3)
            .expect(encode_err_msg);

        assert!(encoded_integer1 < encoded_integer2);
        assert!(encoded_integer2 < encoded_integer3);

        // Show that the domain of negative integers maintain sort order after encoding
        let integer1 = Value::Integer(Integer::from(-1));
        let integer2 = Value::Integer(Integer::from(-600));
        let integer3 = Value::Integer(Integer::from(i64::MIN));

        let encoded_integer1 = &DocumentFieldType::Integer
            .encode_value_for_tree_keys(&integer1)
            .expect(encode_err_msg);
        let encoded_integer2 = &DocumentFieldType::Integer
            .encode_value_for_tree_keys(&integer2)
            .expect(encode_err_msg);
        let encoded_integer3 = &DocumentFieldType::Integer
            .encode_value_for_tree_keys(&integer3)
            .expect(encode_err_msg);

        assert!(encoded_integer1 > encoded_integer2);
        assert!(encoded_integer2 > encoded_integer3);

        // Show that zero is smack in the middle
        let integer1 = Value::Integer(Integer::from(-1));
        let integer2 = Value::Integer(Integer::from(0));
        let integer3 = Value::Integer(Integer::from(1));

        let encoded_integer1 = &DocumentFieldType::Integer
            .encode_value_for_tree_keys(&integer1)
            .expect(encode_err_msg);
        let encoded_integer2 = &DocumentFieldType::Integer
            .encode_value_for_tree_keys(&integer2)
            .expect(encode_err_msg);
        let encoded_integer3 = &DocumentFieldType::Integer
            .encode_value_for_tree_keys(&integer3)
            .expect(encode_err_msg);

        assert!(encoded_integer2 > encoded_integer1);
        assert!(encoded_integer2 < encoded_integer3);

        // Test the relationship between positive and negative integers
        // Since it has been shown that positive integers and negative integers maintain sort order
        // If the smallest positive number is greater than the largest negative number
        // then the positive domain is greater than the negative domain
        // Smallest positive integer is 1 and largest negative integer is -1
        assert!(encoded_integer3 > encoded_integer1);

        // Float encoding
        // Test approach
        // Test positive domain
        // Test negative domain
        // Test against 0
        // Test relationship between positive and negative domain

        // Show that the domain of positive floats maintains sort order after encoding
        let float1 = Value::Float(1.0);
        let float2 = Value::Float(23.65);
        let float3 = Value::Float(1394.584);
        let float4 = Value::Float(f64::MAX);
        let float5 = Value::Float(f64::INFINITY);

        let encoded_number1 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float1)
            .expect(encode_err_msg);
        let encoded_number2 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float2)
            .expect(encode_err_msg);
        let encoded_number3 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float3)
            .expect(encode_err_msg);
        let encoded_number4 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float4)
            .expect(encode_err_msg);
        let encoded_number5 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float5)
            .expect(encode_err_msg);
        let encoded_number6 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&integer1)
            .expect(encode_err_msg);
        let encoded_number7 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&integer2)
            .expect(encode_err_msg);
        let encoded_number8 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&integer3)
            .expect(encode_err_msg);

        assert!(encoded_number1 < encoded_number2);
        assert!(encoded_number2 < encoded_number3);
        assert!(encoded_number3 < encoded_number4);
        assert!(encoded_number4 < encoded_number5);
        assert!(encoded_number6 < encoded_number1);
        assert!(encoded_number7 < encoded_number2);
        assert!(encoded_number7 < encoded_number2);
        assert!(encoded_number8 < encoded_number4);

        // Show that the domain of negative floats maintains sort order after encoding
        let float1 = Value::Float(-0.5);
        let float2 = Value::Float(-23.65);
        let float3 = Value::Float(-1394.584);
        let float4 = Value::Float(f64::MIN);
        let float5 = Value::Float(f64::NEG_INFINITY);

        let encoded_float1 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float1)
            .expect(encode_err_msg);
        let encoded_float2 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float2)
            .expect(encode_err_msg);
        let encoded_float3 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float3)
            .expect(encode_err_msg);
        let encoded_float4 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float4)
            .expect(encode_err_msg);
        let encoded_float5 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float5)
            .expect(encode_err_msg);

        assert!(encoded_float1 > encoded_float2);
        assert!(encoded_float2 > encoded_float3);
        assert!(encoded_float3 > encoded_float4);
        assert!(encoded_float4 > encoded_float5);
        assert!(encoded_float1 < encoded_number8);

        // Show that 0 is in the middle
        // EPSILON: This is the difference between 1.0 and the next larger representable number.
        let largest_negative_float = Value::Float(0.0 - f64::EPSILON);
        let float2 = Value::Float(0.0);
        let smallest_positive_float = Value::Float(0.0 + f64::EPSILON);

        let encoded_float1 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&largest_negative_float)
            .expect(encode_err_msg);
        let encoded_float2 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&float2)
            .expect(encode_err_msg);
        let encoded_float3 = &DocumentFieldType::Number
            .encode_value_for_tree_keys(&smallest_positive_float)
            .expect(encode_err_msg);

        assert!(encoded_float1 < encoded_float2);
        assert!(encoded_float2 < encoded_float3);

        // Test the relationship between positive and negative integers
        // Since it has been shown that positive integers and negative integers maintain sort order
        // If the smallest positive number is greater than the largest negative number
        // then the positive domain is greater than the negative domain
        assert!(encoded_float3 > encoded_float1);

        // Objects should error

        let object_value = Value::Map(vec![(smallest_positive_float, integer1)]);

        let encoded_object =
            &DocumentFieldType::Object(BTreeMap::new()).encode_value_for_tree_keys(&object_value);

        assert!(encoded_object.is_err());
    }
}
