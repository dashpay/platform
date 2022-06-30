use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::option::Option::None;
use std::path::Path;

use byteorder::{BigEndian, WriteBytesExt};
use ciborium::value::Value;

use crate::util::cbor_value::get_key_from_cbor_map;
use crate::ProtocolError;

pub fn json_document_to_cbor(path: impl AsRef<Path>, protocol_version: Option<u32>) -> Vec<u8> {
    let file = File::open(path).expect("file not found");
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader).expect("expected a valid json");
    value_to_cbor(json, protocol_version)
}

pub fn value_to_cbor(value: serde_json::Value, protocol_version: Option<u32>) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();
    if let Some(protocol_version) = protocol_version {
        buffer
            .write_u32::<BigEndian>(protocol_version)
            .expect("writing protocol version caused error");
    }
    ciborium::ser::into_writer(&value, &mut buffer).expect("unable to serialize into cbor");
    buffer
}

pub fn cbor_from_hex(hex_string: String) -> Vec<u8> {
    hex::decode(hex_string).expect("Decoding failed")
}

pub fn text_file_strings(path: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(path).expect("file not found");
    let reader = io::BufReader::new(file).lines();
    reader.into_iter().map(|a| a.unwrap()).collect()
}

pub fn cbor_map_to_btree_map(cbor_map: &[(Value, Value)]) -> BTreeMap<String, &Value> {
    cbor_map
        .iter()
        .filter_map(|(key, value)| key.as_text().map(|key| (key.to_string(), value)))
        .collect::<BTreeMap<String, &Value>>()
}

pub fn cbor_owned_map_to_btree_map(cbor_map: Vec<(Value, Value)>) -> BTreeMap<String, Value> {
    cbor_map
        .into_iter()
        .filter_map(|(key, value)| {
            if let Value::Text(key) = key {
                Some((key, value))
            } else {
                None
            }
        })
        .collect::<BTreeMap<String, Value>>()
}

pub fn cbor_inner_array_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a Vec<Value>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Array(key_value) = key_value {
        return Some(key_value);
    }
    None
}

pub fn cbor_inner_array_of_strings<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<BTreeSet<String>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Array(key_value) = key_value {
        Some(
            key_value
                .iter()
                .filter_map(|v| {
                    if let Value::Text(text) = v {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .collect(),
        )
    } else {
        None
    }
}

pub fn cbor_inner_map_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a Vec<(Value, Value)>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Map(map_value) = key_value {
        return Some(map_value);
    }
    None
}

pub fn cbor_inner_btree_map<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<BTreeMap<String, &'a Value>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Map(map_value) = key_value {
        return Some(cbor_map_to_btree_map(map_value));
    }
    None
}

pub fn btree_map_inner_btree_map<'a>(
    document_type: &'a BTreeMap<String, &'a Value>,
    key: &'a str,
) -> Option<BTreeMap<String, &'a Value>> {
    let key_value = document_type.get(key)?;
    if let Value::Map(map_value) = key_value {
        return Some(cbor_map_to_btree_map(map_value));
    }
    None
}

pub fn btree_map_inner_map_value<'a>(
    document_type: &'a BTreeMap<String, &'a Value>,
    key: &'a str,
) -> Option<&'a Vec<(Value, Value)>> {
    let key_value = document_type.get(key)?;
    if let Value::Map(map_value) = key_value {
        return Some(map_value);
    }
    None
}

pub fn cbor_inner_text_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a str> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Text(string_value) = key_value {
        return Some(string_value);
    }
    None
}

pub fn btree_map_inner_text_value<'a>(
    document_type: &'a BTreeMap<String, &'a Value>,
    key: &'a str,
) -> Option<&'a str> {
    let key_value = document_type.get(key)?;
    if let Value::Text(string_value) = key_value {
        return Some(string_value);
    }
    None
}

pub fn cbor_inner_bytes_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<Vec<u8>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    match key_value {
        Value::Bytes(bytes) => Some(bytes.clone()),
        Value::Array(array) => {
            match array
                .iter()
                .map(|byte| match byte {
                    Value::Integer(int) => {
                        let value_as_u8: u8 = (*int).try_into().map_err(|_| {
                            ProtocolError::DecodingError(String::from("expected u8 value"))
                        })?;
                        Ok(value_as_u8)
                    }
                    _ => Err(ProtocolError::DecodingError(String::from(
                        "not an array of integers",
                    ))),
                })
                .collect::<Result<Vec<u8>, ProtocolError>>()
            {
                Ok(bytes) => Some(bytes),
                Err(_) => None,
            }
        }
        _ => None,
    }
}

pub fn cbor_inner_bool_value(document_type: &[(Value, Value)], key: &str) -> Option<bool> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Bool(bool_value) = key_value {
        return Some(*bool_value);
    }
    None
}

pub fn btree_map_inner_bool_value(
    document_type: &BTreeMap<String, &Value>,
    key: &str,
) -> Option<bool> {
    let key_value = document_type.get(key)?;
    if let Value::Bool(bool_value) = key_value {
        return Some(*bool_value);
    }
    None
}

pub fn cbor_inner_size_value(document_type: &[(Value, Value)], key: &str) -> Option<usize> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Integer(integer) = key_value {
        let value_as_usize: Result<usize, ProtocolError> = (*integer)
            .try_into()
            .map_err(|_| ProtocolError::DecodingError(String::from("expected u8 value")));
        match value_as_usize {
            Ok(size) => Some(size),
            Err(_) => None,
        }
    } else {
        None
    }
}

pub fn btree_map_inner_size_value(
    document_type: &BTreeMap<String, &Value>,
    key: &str,
) -> Option<usize> {
    let key_value = document_type.get(key)?;
    if let Value::Integer(integer) = key_value {
        let value_as_usize: Result<usize, ProtocolError> = (*integer)
            .try_into()
            .map_err(|_| ProtocolError::DecodingError(String::from("expected u8 value")));
        match value_as_usize {
            Ok(size) => Some(size),
            Err(_) => None,
        }
    } else {
        None
    }
}

pub fn cbor_inner_bool_value_with_default(
    document_type: &[(Value, Value)],
    key: &str,
    default: bool,
) -> bool {
    cbor_inner_bool_value(document_type, key).unwrap_or(default)
}

pub fn bytes_for_system_value(value: &Value) -> Result<Option<Vec<u8>>, ProtocolError> {
    match value {
        Value::Bytes(bytes) => Ok(Some(bytes.clone())),
        Value::Text(text) => match bs58::decode(text).into_vec() {
            Ok(data) => Ok(Some(data)),
            Err(_) => Ok(None),
        },
        Value::Array(array) => array
            .iter()
            .map(|byte| match byte {
                Value::Integer(int) => {
                    let value_as_u8: u8 = (*int).try_into().map_err(|_| {
                        ProtocolError::DecodingError(String::from("expected u8 value"))
                    })?;
                    Ok(Some(value_as_u8))
                }
                _ => Err(ProtocolError::DecodingError(String::from(
                    "not an array of integers",
                ))),
            })
            .collect::<Result<Option<Vec<u8>>, ProtocolError>>(),
        _ => Err(ProtocolError::DecodingError(String::from(
            "system value is incorrect type",
        ))),
    }
}

pub fn bytes_for_system_value_from_tree_map(
    document: &BTreeMap<String, Value>,
    key: &str,
) -> Result<Option<Vec<u8>>, ProtocolError> {
    let value = document.get(key);
    if let Some(value) = value {
        bytes_for_system_value(value)
    } else {
        Ok(None)
    }
}

pub(crate) fn cbor_inner_u64_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<u64> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Integer(integer_value) = key_value {
        return Some(i128::from(*integer_value) as u64);
    }
    None
}

pub(crate) fn cbor_inner_u32_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<u32> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Integer(integer_value) = key_value {
        return Some(i128::from(*integer_value) as u32);
    }
    None
}
