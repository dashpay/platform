use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use integer_encoding::VarInt;
use platform_value::Value;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum ArrayItemType {
    Integer,
    Number,
    String(Option<usize>, Option<usize>),
    ByteArray(Option<usize>, Option<usize>),
    Identifier,
    Boolean,
    Date,
}

impl ArrayItemType {
    pub fn encode_value_with_size(&self, value: Value) -> Result<Vec<u8>, ProtocolError> {
        match self {
            ArrayItemType::String(_, _) => {
                if let Value::Text(value) = value {
                    let vec = value.into_bytes();
                    let mut r_vec = vec.len().encode_var_vec();
                    r_vec.extend(vec);
                    Ok(r_vec)
                } else {
                    Err(get_field_type_matching_error())
                }
            }
            ArrayItemType::Date => {
                let value_as_f64 = value.into_float().map_err(ProtocolError::ValueError)?;
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::Integer => {
                let value_as_i64: i64 = value.into_integer().map_err(ProtocolError::ValueError)?;

                let value_bytes = value_as_i64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::Number => {
                let value_as_f64 = value.into_float().map_err(ProtocolError::ValueError)?;
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::ByteArray(_, _) => {
                let mut bytes = value.into_binary_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            ArrayItemType::Identifier => {
                let mut bytes = value.into_identifier_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            ArrayItemType::Boolean => {
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
                if value_as_boolean {
                    Ok(vec![1]) // 1 is true
                } else {
                    Ok(vec![0]) // 2 is false
                }
            }
        }
    }

    pub fn encode_value_ref_with_size(&self, value: &Value) -> Result<Vec<u8>, ProtocolError> {
        return match self {
            ArrayItemType::String(_, _) => {
                let value_as_text = value.as_text().ok_or_else(get_field_type_matching_error)?;
                let vec = value_as_text.as_bytes().to_vec();
                let mut r_vec = vec.len().encode_var_vec();
                r_vec.extend(vec);
                Ok(r_vec)
            }
            ArrayItemType::Date => {
                let value_as_f64 = value.to_float().map_err(ProtocolError::ValueError)?;
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::Integer => {
                let value_as_i64: i64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                let value_bytes = value_as_i64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::Number => {
                let value_as_f64 = value.to_float().map_err(ProtocolError::ValueError)?;
                let value_bytes = value_as_f64.to_be_bytes().to_vec();
                Ok(value_bytes)
            }
            ArrayItemType::ByteArray(_, _) => {
                let mut bytes = value.to_binary_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            ArrayItemType::Identifier => {
                let mut bytes = value.to_identifier_bytes()?;

                let mut r_vec = bytes.len().encode_var_vec();
                r_vec.append(&mut bytes);
                Ok(r_vec)
            }
            ArrayItemType::Boolean => {
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

fn get_field_type_matching_error() -> ProtocolError {
    ProtocolError::DataContractError(DataContractError::ValueWrongType(
        "document field type doesn't match document value for array".to_string(),
    ))
}
