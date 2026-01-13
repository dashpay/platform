use crate::data_contract::document_type::DocumentPropertyType;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use integer_encoding::{VarInt, VarIntReader};
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Read};

/// Maximum string length allowed during deserialization of array elements.
/// This prevents DoS attacks via huge length values in corrupted/malicious data.
pub const MAX_STRING_LENGTH_FOR_DESERIALIZATION: usize = 65536; // 64 KB

/// Maximum byte array length allowed during deserialization of array elements.
/// This prevents DoS attacks via huge length values in corrupted/malicious data.
pub const MAX_BYTE_ARRAY_LENGTH_FOR_DESERIALIZATION: usize = 65536; // 64 KB

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

    /// Encodes an array element value for use as an index tree key.
    /// This encoding is compatible with the scalar type encoding used for tree keys.
    /// Unlike `encode_value_ref_with_size`, this does NOT prepend a length prefix,
    /// making it suitable for index key comparisons and tree traversal.
    pub fn encode_element_for_tree_keys(&self, value: &Value) -> Result<Vec<u8>, ProtocolError> {
        if value.is_null() {
            return Ok(vec![]);
        }
        match self {
            ArrayItemType::String(_, _) => {
                let value_as_text = value.as_text().ok_or_else(get_field_type_matching_error)?;
                let vec = value_as_text.as_bytes().to_vec();
                if vec.is_empty() {
                    // we don't want to collide with the definition of an empty string
                    Ok(vec![0])
                } else {
                    Ok(vec)
                }
            }
            ArrayItemType::Date => {
                let value_as_i64: i64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                if value_as_i64 < 0 {
                    return Err(ProtocolError::DataContractError(
                        DataContractError::ValueWrongType(
                            "date timestamp cannot be negative".to_string(),
                        ),
                    ));
                }
                // Use the same encoding as DocumentPropertyType::encode_date_timestamp
                // which uses encode_u64 with sign-bit flip for proper lexicographic ordering
                Ok(DocumentPropertyType::encode_date_timestamp(
                    value_as_i64 as u64,
                ))
            }
            ArrayItemType::Integer => {
                let value_as_i64: i64 = value.to_integer().map_err(ProtocolError::ValueError)?;
                // Use encode_i64 which flips sign bit for proper lexicographic ordering
                Ok(DocumentPropertyType::encode_i64(value_as_i64))
            }
            ArrayItemType::Number => {
                let value_as_f64 = value.to_float().map_err(ProtocolError::ValueError)?;
                // Use encode_float which handles sign bit and negative value ordering
                Ok(DocumentPropertyType::encode_float(value_as_f64))
            }
            ArrayItemType::ByteArray(_, _) => {
                let bytes = value.to_binary_bytes()?;
                if bytes.is_empty() {
                    // we don't want to collide with the definition of null
                    Ok(vec![0])
                } else {
                    Ok(bytes)
                }
            }
            ArrayItemType::Identifier => {
                let bytes = value.to_identifier_bytes()?;
                Ok(bytes)
            }
            ArrayItemType::Boolean => {
                let value_as_boolean = value.as_bool().ok_or_else(get_field_type_matching_error)?;
                if value_as_boolean {
                    Ok(vec![1])
                } else {
                    Ok(vec![0])
                }
            }
        }
    }

    /// Reads a single array element value from a buffer.
    /// This is the inverse of `encode_value_ref_with_size`.
    pub fn read_from<R: Read + BufRead>(&self, buf: &mut R) -> Result<Value, DataContractError> {
        match self {
            ArrayItemType::String(_, _) => {
                let string_len: usize = buf.read_varint().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading varint for string length in array".to_string(),
                    )
                })?;
                // Validate string length to prevent DoS via huge allocations
                if string_len > MAX_STRING_LENGTH_FOR_DESERIALIZATION {
                    return Err(DataContractError::CorruptedSerialization(format!(
                        "string length {} exceeds maximum allowed {}",
                        string_len, MAX_STRING_LENGTH_FOR_DESERIALIZATION
                    )));
                }
                let mut string_bytes = vec![0u8; string_len];
                buf.read_exact(&mut string_bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading string bytes in array".to_string(),
                    )
                })?;
                let string_value = String::from_utf8(string_bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "invalid UTF-8 in array string".to_string(),
                    )
                })?;
                Ok(Value::Text(string_value))
            }
            ArrayItemType::Date => {
                let mut date_bytes = [0u8; 8];
                buf.read_exact(&mut date_bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading date bytes in array".to_string(),
                    )
                })?;
                let date_value = f64::from_be_bytes(date_bytes);
                Ok(Value::Float(date_value))
            }
            ArrayItemType::Integer => {
                let mut int_bytes = [0u8; 8];
                buf.read_exact(&mut int_bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading integer bytes in array".to_string(),
                    )
                })?;
                let int_value = i64::from_be_bytes(int_bytes);
                Ok(Value::I64(int_value))
            }
            ArrayItemType::Number => {
                let mut num_bytes = [0u8; 8];
                buf.read_exact(&mut num_bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading number bytes in array".to_string(),
                    )
                })?;
                let num_value = f64::from_be_bytes(num_bytes);
                Ok(Value::Float(num_value))
            }
            ArrayItemType::ByteArray(_, _) => {
                let bytes_len: usize = buf.read_varint().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading varint for byte array length in array".to_string(),
                    )
                })?;
                // Validate byte array length to prevent DoS via huge allocations
                if bytes_len > MAX_BYTE_ARRAY_LENGTH_FOR_DESERIALIZATION {
                    return Err(DataContractError::CorruptedSerialization(format!(
                        "byte array length {} exceeds maximum allowed {}",
                        bytes_len, MAX_BYTE_ARRAY_LENGTH_FOR_DESERIALIZATION
                    )));
                }
                let mut bytes = vec![0u8; bytes_len];
                buf.read_exact(&mut bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading byte array bytes in array".to_string(),
                    )
                })?;
                Ok(Value::Bytes(bytes))
            }
            ArrayItemType::Identifier => {
                let id_len: usize = buf.read_varint().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading varint for identifier length in array".to_string(),
                    )
                })?;
                if id_len != 32 {
                    return Err(DataContractError::CorruptedSerialization(format!(
                        "expected 32 bytes for identifier in array, got {}",
                        id_len
                    )));
                }
                let mut id_bytes = [0u8; 32];
                buf.read_exact(&mut id_bytes).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading identifier bytes in array".to_string(),
                    )
                })?;
                Ok(Value::Identifier(id_bytes))
            }
            ArrayItemType::Boolean => {
                let mut bool_byte = [0u8; 1];
                buf.read_exact(&mut bool_byte).map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading boolean byte in array".to_string(),
                    )
                })?;
                Ok(Value::Bool(bool_byte[0] != 0))
            }
        }
    }
}

fn get_field_type_matching_error() -> ProtocolError {
    ProtocolError::DataContractError(DataContractError::ValueWrongType(
        "document field type doesn't match document value for array".to_string(),
    ))
}
