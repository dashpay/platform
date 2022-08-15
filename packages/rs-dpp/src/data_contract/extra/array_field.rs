use std::convert::TryInto;

use ciborium::value::Value;
use integer_encoding::VarInt;
use serde::{Deserialize, Serialize};

use super::errors::ContractError;

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
    pub fn encode_value_with_size(&self, value: Value) -> Result<Vec<u8>, ContractError> {
        match self {
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
                        let value_as_i128: i128 = value_as_integer
                            .try_into()
                            .map_err(|_| ContractError::ValueWrongType("expected integer value"))?;
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

                let value_as_i64: i64 = value_as_integer
                    .try_into()
                    .map_err(|_| ContractError::ValueWrongType("expected integer value"))?;
                let value_bytes = value_as_i64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayFieldType::Number => {
                let value_as_f64 = if value.is_integer() {
                    let value_as_integer = value
                        .as_integer()
                        .ok_or_else(get_field_type_matching_error)?;

                    let value_as_i64: i64 = value_as_integer
                        .try_into()
                        .map_err(|_| ContractError::ValueWrongType("expected number value"))?;

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
                            ContractError::ValueDecodingError("bytearray: invalid base64 value")
                        })?;
                        Ok(value_as_bytes)
                    }
                    Value::Array(array) => array
                        .into_iter()
                        .map(|byte| match byte {
                            Value::Integer(int) => {
                                let value_as_u8: u8 = int.try_into().map_err(|_| {
                                    ContractError::ValueWrongType("expected u8 value")
                                })?;
                                Ok(value_as_u8)
                            }
                            _ => Err(ContractError::ValueWrongType("not an array of integers")),
                        })
                        .collect::<Result<Vec<u8>, ContractError>>(),
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
        }
    }

    pub fn encode_value_ref_with_size(&self, value: &Value) -> Result<Vec<u8>, ContractError> {
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
                        let value_as_i128: i128 = value_as_integer
                            .try_into()
                            .map_err(|_| ContractError::ValueWrongType("expected integer value"))?;
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

                let value_as_i64: i64 = value_as_integer
                    .try_into()
                    .map_err(|_| ContractError::ValueWrongType("expected integer value"))?;
                let value_bytes = value_as_i64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayFieldType::Number => {
                let value_as_f64 = if value.is_integer() {
                    let value_as_integer = value
                        .as_integer()
                        .ok_or_else(get_field_type_matching_error)?;

                    let value_as_i64: i64 = value_as_integer
                        .try_into()
                        .map_err(|_| ContractError::ValueWrongType("expected number value"))?;

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
                            ContractError::ValueDecodingError("bytearray: invalid base64 value")
                        })?;
                        Ok(value_as_bytes)
                    }
                    Value::Array(array) => array
                        .iter()
                        .map(|byte| match byte {
                            Value::Integer(int) => {
                                let value_as_u8: u8 = (*int).try_into().map_err(|_| {
                                    ContractError::ValueWrongType("expected u8 value")
                                })?;
                                Ok(value_as_u8)
                            }
                            _ => Err(ContractError::ValueWrongType("not an array of integers")),
                        })
                        .collect::<Result<Vec<u8>, ContractError>>(),
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

fn get_field_type_matching_error() -> ContractError {
    ContractError::ValueWrongType("document field type doesn't match document value")
}
