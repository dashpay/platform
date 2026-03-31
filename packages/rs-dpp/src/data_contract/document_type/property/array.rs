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
    /// Sanitize a value to match the expected array item type
    pub fn sanitize_value_mut(&self, value: &mut Value) {
        match (self, value.clone()) {
            // Convert hex or base64 strings to byte arrays for ByteArray items
            (ArrayItemType::ByteArray(min_size, max_size), Value::Text(str_value)) => {
                // Try to decode the string
                let decoded_bytes = if let Ok(bytes) = hex::decode(str_value.as_str()) {
                    Some(bytes)
                } else {
                    // If hex fails, try base64 decoding
                    use base64::{engine::general_purpose, Engine as _};
                    general_purpose::STANDARD.decode(str_value.as_str()).ok()
                };

                if let Some(bytes) = decoded_bytes {
                    let byte_len = bytes.len();

                    // Check if the decoded bytes meet the size constraints
                    let size_ok = match (*min_size, *max_size) {
                        (Some(min), Some(max)) => byte_len >= min && byte_len <= max,
                        (Some(min), None) => byte_len >= min,
                        (None, Some(max)) => byte_len <= max,
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

            // Convert hex or base58 strings to identifiers for Identifier items
            (ArrayItemType::Identifier, Value::Text(str_value)) => {
                use platform_value::Identifier;
                // First try base58 decoding (most common for identifiers)
                if let Ok(id) = Identifier::from_string(
                    &str_value,
                    platform_value::string_encoding::Encoding::Base58,
                ) {
                    *value = Value::Identifier(id.into_buffer());
                } else {
                    // If base58 fails, try hex decoding
                    // Remove any spaces or non-hex characters
                    let clean_hex: String = str_value
                        .chars()
                        .filter(|c| c.is_ascii_hexdigit())
                        .collect();

                    // Try to decode hex string to identifier
                    if clean_hex.len() == 64 {
                        // 32 bytes = 64 hex chars
                        if let Ok(bytes) = hex::decode(&clean_hex) {
                            if let Ok(id) = Identifier::try_from(bytes.as_slice()) {
                                *value = Value::Identifier(id.into_buffer());
                            }
                        }
                    }
                }
                // If both conversions fail, leave the value as is (validation will catch it later)
            }

            // Convert positive I64 to U64 for Date items
            (ArrayItemType::Date, Value::I64(timestamp)) if timestamp >= 0 => {
                *value = Value::U64(timestamp as u64);
            }

            // Ensure integers are converted properly
            (ArrayItemType::Integer, Value::U64(n)) if n <= i64::MAX as u64 => {
                *value = Value::I64(n as i64);
            }
            (ArrayItemType::Integer, Value::U32(n)) => {
                *value = Value::I64(n as i64);
            }
            (ArrayItemType::Integer, Value::U16(n)) => {
                *value = Value::I64(n as i64);
            }
            (ArrayItemType::Integer, Value::U8(n)) => {
                *value = Value::I64(n as i64);
            }

            // Ensure numbers are converted to F64
            (ArrayItemType::Number, Value::I64(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::U64(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::I32(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::U32(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::I16(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::U16(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::I8(n)) => {
                *value = Value::Float(n as f64);
            }
            (ArrayItemType::Number, Value::U8(n)) => {
                *value = Value::Float(n as f64);
            }

            // For all other cases, leave the value as is
            _ => {}
        }
    }

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
        match self {
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
        }
    }
}

fn get_field_type_matching_error() -> ProtocolError {
    ProtocolError::DataContractError(DataContractError::ValueWrongType(
        "document field type doesn't match document value for array".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // encode_value_with_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_with_size_string() {
        let item = ArrayItemType::String(None, None);
        let result = item
            .encode_value_with_size(Value::Text("abc".to_string()))
            .unwrap();
        // varint(3) + b"abc"
        assert_eq!(result.len(), 4);
        assert_eq!(&result[1..], b"abc");
    }

    #[test]
    fn test_encode_value_with_size_string_type_mismatch() {
        let item = ArrayItemType::String(None, None);
        let result = item.encode_value_with_size(Value::U64(42));
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_with_size_integer() {
        let item = ArrayItemType::Integer;
        let result = item.encode_value_with_size(Value::I64(42)).unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result, 42i64.to_be_bytes().to_vec());
    }

    #[test]
    fn test_encode_value_with_size_number() {
        let item = ArrayItemType::Number;
        let result = item.encode_value_with_size(Value::Float(3.14)).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_with_size_date() {
        let item = ArrayItemType::Date;
        let result = item
            .encode_value_with_size(Value::Float(1648910575.0))
            .unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_with_size_byte_array() {
        let item = ArrayItemType::ByteArray(None, None);
        let bytes = vec![1u8, 2, 3, 4];
        let result = item.encode_value_with_size(Value::Bytes(bytes)).unwrap();
        // varint(4) + [1,2,3,4]
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_encode_value_with_size_identifier() {
        let item = ArrayItemType::Identifier;
        let id_bytes = [5u8; 32];
        let result = item
            .encode_value_with_size(Value::Identifier(id_bytes))
            .unwrap();
        // varint(32) + 32 bytes
        assert_eq!(result.len(), 33);
    }

    #[test]
    fn test_encode_value_with_size_boolean_true() {
        let item = ArrayItemType::Boolean;
        let result = item.encode_value_with_size(Value::Bool(true)).unwrap();
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn test_encode_value_with_size_boolean_false() {
        let item = ArrayItemType::Boolean;
        let result = item.encode_value_with_size(Value::Bool(false)).unwrap();
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_encode_value_with_size_boolean_type_mismatch() {
        let item = ArrayItemType::Boolean;
        let result = item.encode_value_with_size(Value::U64(42));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // encode_value_ref_with_size() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_value_ref_with_size_string() {
        let item = ArrayItemType::String(None, None);
        let val = Value::Text("test".to_string());
        let result = item.encode_value_ref_with_size(&val).unwrap();
        assert_eq!(result.len(), 5); // varint(4) + "test"
    }

    #[test]
    fn test_encode_value_ref_with_size_string_type_mismatch() {
        let item = ArrayItemType::String(None, None);
        let val = Value::U64(42);
        let result = item.encode_value_ref_with_size(&val);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_value_ref_with_size_integer() {
        let item = ArrayItemType::Integer;
        let val = Value::I64(-100);
        let result = item.encode_value_ref_with_size(&val).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_number() {
        let item = ArrayItemType::Number;
        let val = Value::Float(2.718);
        let result = item.encode_value_ref_with_size(&val).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_date() {
        let item = ArrayItemType::Date;
        let val = Value::Float(1648910575.0);
        let result = item.encode_value_ref_with_size(&val).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_encode_value_ref_with_size_byte_array() {
        let item = ArrayItemType::ByteArray(None, None);
        let val = Value::Bytes(vec![10, 20, 30]);
        let result = item.encode_value_ref_with_size(&val).unwrap();
        // varint(3) + [10,20,30]
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_encode_value_ref_with_size_identifier() {
        let item = ArrayItemType::Identifier;
        let val = Value::Identifier([7u8; 32]);
        let result = item.encode_value_ref_with_size(&val).unwrap();
        // varint(32) + 32 bytes
        assert_eq!(result.len(), 33);
    }

    #[test]
    fn test_encode_value_ref_with_size_boolean_true() {
        let item = ArrayItemType::Boolean;
        let result = item.encode_value_ref_with_size(&Value::Bool(true)).unwrap();
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn test_encode_value_ref_with_size_boolean_false() {
        let item = ArrayItemType::Boolean;
        let result = item
            .encode_value_ref_with_size(&Value::Bool(false))
            .unwrap();
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_encode_value_ref_with_size_boolean_type_mismatch() {
        let item = ArrayItemType::Boolean;
        let result = item.encode_value_ref_with_size(&Value::U64(42));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // sanitize_value_mut() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanitize_byte_array_from_hex_string() {
        let item = ArrayItemType::ByteArray(None, None);
        let mut val = Value::Text("deadbeef".to_string());
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    }

    #[test]
    fn test_sanitize_byte_array_from_hex_string_exact_20() {
        let item = ArrayItemType::ByteArray(Some(20), Some(20));
        let hex_str = "aa".repeat(20); // 40 hex chars = 20 bytes
        let mut val = Value::Text(hex_str);
        item.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes20(_)));
    }

    #[test]
    fn test_sanitize_byte_array_from_hex_string_exact_32() {
        let item = ArrayItemType::ByteArray(Some(32), Some(32));
        let hex_str = "bb".repeat(32);
        let mut val = Value::Text(hex_str);
        item.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes32(_)));
    }

    #[test]
    fn test_sanitize_byte_array_from_hex_string_exact_36() {
        let item = ArrayItemType::ByteArray(Some(36), Some(36));
        let hex_str = "cc".repeat(36);
        let mut val = Value::Text(hex_str);
        item.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Bytes36(_)));
    }

    #[test]
    fn test_sanitize_byte_array_size_constraint_too_small() {
        let item = ArrayItemType::ByteArray(Some(10), None);
        let mut val = Value::Text("aabb".to_string()); // 2 bytes, min is 10
        item.sanitize_value_mut(&mut val);
        // Should remain unchanged because size constraint is violated
        assert!(matches!(val, Value::Text(_)));
    }

    #[test]
    fn test_sanitize_byte_array_size_constraint_too_big() {
        let item = ArrayItemType::ByteArray(None, Some(2));
        let mut val = Value::Text("aabbccddee".to_string()); // 5 bytes, max is 2
        item.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Text(_)));
    }

    #[test]
    fn test_sanitize_identifier_from_hex_string() {
        let item = ArrayItemType::Identifier;
        let hex_str = "aa".repeat(32);
        let mut val = Value::Text(hex_str);
        item.sanitize_value_mut(&mut val);
        assert!(matches!(val, Value::Identifier(_)));
    }

    #[test]
    fn test_sanitize_date_from_positive_i64() {
        let item = ArrayItemType::Date;
        let mut val = Value::I64(1648910575000);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::U64(1648910575000));
    }

    #[test]
    fn test_sanitize_date_from_negative_i64_unchanged() {
        let item = ArrayItemType::Date;
        let mut val = Value::I64(-1);
        item.sanitize_value_mut(&mut val);
        // Negative timestamps should not be converted
        assert_eq!(val, Value::I64(-1));
    }

    #[test]
    fn test_sanitize_integer_from_u64() {
        let item = ArrayItemType::Integer;
        let mut val = Value::U64(42);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I64(42));
    }

    #[test]
    fn test_sanitize_integer_from_u32() {
        let item = ArrayItemType::Integer;
        let mut val = Value::U32(100);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I64(100));
    }

    #[test]
    fn test_sanitize_integer_from_u16() {
        let item = ArrayItemType::Integer;
        let mut val = Value::U16(300);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I64(300));
    }

    #[test]
    fn test_sanitize_integer_from_u8() {
        let item = ArrayItemType::Integer;
        let mut val = Value::U8(255);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::I64(255));
    }

    #[test]
    fn test_sanitize_number_from_i64() {
        let item = ArrayItemType::Number;
        let mut val = Value::I64(42);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(42.0));
    }

    #[test]
    fn test_sanitize_number_from_u64() {
        let item = ArrayItemType::Number;
        let mut val = Value::U64(100);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(100.0));
    }

    #[test]
    fn test_sanitize_number_from_i32() {
        let item = ArrayItemType::Number;
        let mut val = Value::I32(-50);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(-50.0));
    }

    #[test]
    fn test_sanitize_number_from_u32() {
        let item = ArrayItemType::Number;
        let mut val = Value::U32(200);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(200.0));
    }

    #[test]
    fn test_sanitize_number_from_i16() {
        let item = ArrayItemType::Number;
        let mut val = Value::I16(-10);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(-10.0));
    }

    #[test]
    fn test_sanitize_number_from_u16() {
        let item = ArrayItemType::Number;
        let mut val = Value::U16(500);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(500.0));
    }

    #[test]
    fn test_sanitize_number_from_i8() {
        let item = ArrayItemType::Number;
        let mut val = Value::I8(-5);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(-5.0));
    }

    #[test]
    fn test_sanitize_number_from_u8() {
        let item = ArrayItemType::Number;
        let mut val = Value::U8(7);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Float(7.0));
    }

    #[test]
    fn test_sanitize_leaves_matching_type_unchanged() {
        let item = ArrayItemType::Boolean;
        let mut val = Value::Bool(true);
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Bool(true));
    }

    #[test]
    fn test_sanitize_leaves_unrelated_type_unchanged() {
        let item = ArrayItemType::Integer;
        let mut val = Value::Text("not a number".to_string());
        item.sanitize_value_mut(&mut val);
        assert_eq!(val, Value::Text("not a number".to_string()));
    }
}
