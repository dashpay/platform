use crate::data_contract::errors::StructureError;
use crate::util::cbor_value::{cbor_value_into_json_value, cbor_value_to_json_value};
use crate::util::serializer::value_to_cbor;
use crate::ProtocolError;
use ciborium::Value;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufReader;
use std::iter::FromIterator;
use std::path::Path;

// use std::collections::{BTreeMap, BTreeSet};
// use std::convert::TryInto;
// use std::fs::File;
// use std::io::BufReader;
// use std::path::Path;
//
// use byteorder::{BigEndian, WriteBytesExt};
// use ciborium::value::Value;
//
// use super::errors::StructureError;
//

pub fn cbor_map_into_btree_map(
    cbor_map: Vec<(Value, Value)>,
) -> Result<BTreeMap<String, Value>, ProtocolError> {
    cbor_map
        .into_iter()
        .map(|(key, value)| {
            let key = key.into_text().map_err(|_| {
                ProtocolError::StructureError(StructureError::KeyWrongType(
                    "expected key to be string",
                ))
            })?;
            Ok((key, value))
        })
        .collect::<Result<BTreeMap<String, Value>, ProtocolError>>()
}

//todo remove this function
pub fn cbor_map_into_serde_btree_map(
    cbor_map: Vec<(Value, Value)>,
) -> Result<BTreeMap<String, serde_json::Value>, ProtocolError> {
    cbor_map
        .into_iter()
        .map(|(key, value)| {
            let key = key.into_text().map_err(|_| {
                ProtocolError::StructureError(StructureError::KeyWrongType(
                    "expected key to be string",
                ))
            })?;
            let value = cbor_value_into_json_value(value)?;
            Ok((key, value))
        })
        .collect::<Result<BTreeMap<String, serde_json::Value>, ProtocolError>>()
}

/// Converts a CBOR map to a BTree map.
pub fn cbor_map_to_btree_map(cbor_map: &[(Value, Value)]) -> BTreeMap<String, &Value> {
    cbor_map
        .iter()
        .filter_map(|(key, value)| key.as_text().map(|key| (key.to_string(), value)))
        .collect::<BTreeMap<String, &Value>>()
}

/// Gets the inner bool value from cbor map
pub fn cbor_inner_bool_value(document_type: &[(Value, Value)], key: &str) -> Option<bool> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Bool(bool_value) = key_value {
        return Some(*bool_value);
    }
    None
}

/// Gets the inner array value from cbor map
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

pub fn get_key_from_cbor_map<'a>(
    cbor_map: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a Value> {
    for (cbor_key, cbor_value) in cbor_map.iter() {
        if !cbor_key.is_text() {
            continue;
        }

        if cbor_key.as_text().expect("confirmed as text") == key {
            return Some(cbor_value);
        }
    }
    None
}

/// Retrieves the value of a key from a CBOR map if it's an array of strings.
pub fn cbor_inner_array_of_strings<'a, I: FromIterator<String>>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<I> {
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
//
// pub fn cbor_inner_map_value<'a>(
//     document_type: &'a [(Value, Value)],
//     key: &'a str,
// ) -> Option<&'a Vec<(Value, Value)>> {
//     let key_value = get_key_from_cbor_map(document_type, key)?;
//     if let Value::Map(map_value) = key_value {
//         return Some(map_value);
//     }
//     None
// }
//
/// Retrieves the value of a key from a CBOR map, and if it's a map itself,
/// returns it as a B-tree map.
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
//
/// Retrieves the value of a key from a B-tree map, and if it's a map itself,
/// returns it as a B-tree map.
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

/// Retrieves the value of a key from a B-tree map if it's a map itself.
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

/// Retrieves the value of a key from a CBOR map if it's a string.
pub fn cbor_inner_text_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Result<Option<&'a str>, ProtocolError> {
    match get_key_from_cbor_map(document_type, key) {
        None => Ok(None),
        Some(key_value) => {
            if let Value::Text(string_value) = key_value {
                Ok(Some(string_value))
            } else {
                Err(ProtocolError::StructureError(
                    StructureError::ValueWrongType("expected a string for the value"),
                ))
            }
        }
    }
}

/// Retrieves the value of a key from a CBOR map if it's a byte array.
pub fn cbor_inner_bytes_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Result<Option<Vec<u8>>, ProtocolError> {
    match get_key_from_cbor_map(document_type, key) {
        None => Ok(None),
        Some(key_value) => match key_value {
            Value::Bytes(bytes) => Ok(Some(bytes.clone())),
            Value::Array(array) => array
                .iter()
                .map(|byte| match byte {
                    Value::Integer(int) => {
                        let value_as_u8: u8 = (*int).try_into().map_err(|_| {
                            ProtocolError::StructureError(StructureError::ValueWrongType(
                                "expected u8 value",
                            ))
                        })?;
                        Ok(value_as_u8)
                    }
                    _ => Err(ProtocolError::StructureError(
                        StructureError::ValueWrongType("not an array of integers"),
                    )),
                })
                .collect::<Result<Vec<u8>, ProtocolError>>()
                .map(|v| Some(v)),
            _ => Err(ProtocolError::StructureError(
                StructureError::ValueWrongType("value should be a byte array"),
            )),
        },
    }
}

/// Takes a value (should be a system value) and returns it as a byte array if possible.
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
                        ProtocolError::StructureError(StructureError::ValueWrongType(
                            "expected u8 value",
                        ))
                    })?;
                    Ok(Some(value_as_u8))
                }
                _ => Err(ProtocolError::StructureError(
                    StructureError::ValueWrongType("not an array of integers"),
                )),
            })
            .collect::<Result<Option<Vec<u8>>, ProtocolError>>(),
        _ => Err(ProtocolError::StructureError(
            StructureError::ValueWrongType("system value is incorrect type"),
        )),
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

/// Reads a JSON file and converts it to CBOR.
pub fn json_document_to_cbor(
    path: impl AsRef<Path>,
    protocol_version: Option<u32>,
) -> Result<Vec<u8>, ProtocolError> {
    let file = File::open(path).expect("file not found");

    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader).expect("expected a valid json");
    value_to_cbor(json, protocol_version)
}
//
// pub fn value_to_cbor(value: serde_json::Value, protocol_version: Option<u32>) -> Vec<u8> {
//     let mut buffer: Vec<u8> = Vec::new();
//     if let Some(protocol_version) = protocol_version {
//         buffer
//             .write_u32::<BigEndian>(protocol_version)
//             .expect("writing protocol version caused error");
//     }
//     ciborium::ser::into_writer(&value, &mut buffer).expect("unable to serialize into cbor");
//     buffer
// }
//

/// Make sure the protocol version is correct.
pub const fn check_protocol_version(_version: u32) -> bool {
    // Temporary disabled due protocol version is dynamic and goes from consensus params
    true
}

/// Makes sure the protocol version is correct given the version as a u8.
pub fn check_protocol_version_bytes(version_bytes: &[u8]) -> bool {
    if version_bytes.len() != 4 {
        false
    } else {
        let version_set_bytes: [u8; 4] = version_bytes
            .try_into()
            .expect("slice with incorrect length");
        let version = u32::from_be_bytes(version_set_bytes);
        check_protocol_version(version)
    }
}

pub fn reduced_value_string_representation(value: &Value) -> String {
    match value {
        Value::Integer(integer) => {
            let i: i128 = (*integer).try_into().unwrap();
            format!("{}", i)
        }
        Value::Bytes(bytes) => hex::encode(bytes),
        Value::Float(float) => {
            format!("{}", float)
        }
        Value::Text(text) => {
            let len = text.len();
            if len > 20 {
                let first_text = text.split_at(20).0.to_string();
                format!("{}[...({})]", first_text, len)
            } else {
                text.clone()
            }
        }
        Value::Bool(b) => {
            format!("{}", b)
        }
        Value::Null => "None".to_string(),
        Value::Tag(_, _) => "Tag".to_string(),
        Value::Array(_) => "Array".to_string(),
        Value::Map(_) => "Map".to_string(),
        _ => "".to_string(),
    }
}
