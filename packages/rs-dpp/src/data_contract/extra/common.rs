use crate::data_contract::errors::StructureError;
use crate::util::serializer::value_to_cbor;
use crate::ProtocolError;
use ciborium::Value;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufReader;
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

pub fn cbor_map_to_btree_map(cbor_map: &[(Value, Value)]) -> BTreeMap<String, &Value> {
    cbor_map
        .iter()
        .filter_map(|(key, value)| key.as_text().map(|key| (key.to_string(), value)))
        .collect::<BTreeMap<String, &Value>>()
}
//
// pub fn cbor_inner_array_value<'a>(
//     document_type: &'a [(Value, Value)],
//     key: &'a str,
// ) -> Option<&'a Vec<Value>> {
//     let key_value = get_key_from_cbor_map(document_type, key)?;
//     if let Value::Array(key_value) = key_value {
//         return Some(key_value);
//     }
//     None
// }
//
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
//
// pub fn cbor_inner_array_of_strings<'a>(
//     document_type: &'a [(Value, Value)],
//     key: &'a str,
// ) -> Option<BTreeSet<String>> {
//     let key_value = get_key_from_cbor_map(document_type, key)?;
//     if let Value::Array(key_value) = key_value {
//         Some(
//             key_value
//                 .iter()
//                 .filter_map(|v| {
//                     if let Value::Text(text) = v {
//                         Some(text.clone())
//                     } else {
//                         None
//                     }
//                 })
//                 .collect(),
//         )
//     } else {
//         None
//     }
// }
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
// pub fn cbor_inner_btree_map<'a>(
//     document_type: &'a [(Value, Value)],
//     key: &'a str,
// ) -> Option<BTreeMap<String, &'a Value>> {
//     let key_value = get_key_from_cbor_map(document_type, key)?;
//     if let Value::Map(map_value) = key_value {
//         return Some(cbor_map_to_btree_map(map_value));
//     }
//     None
// }
//
// pub fn btree_map_inner_btree_map<'a>(
//     document_type: &'a BTreeMap<String, &'a Value>,
//     key: &'a str,
// ) -> Option<BTreeMap<String, &'a Value>> {
//     let key_value = document_type.get(key)?;
//     if let Value::Map(map_value) = key_value {
//         return Some(cbor_map_to_btree_map(map_value));
//     }
//     None
// }
//
// pub fn btree_map_inner_map_value<'a>(
//     document_type: &'a BTreeMap<String, &'a Value>,
//     key: &'a str,
// ) -> Option<&'a Vec<(Value, Value)>> {
//     let key_value = document_type.get(key)?;
//     if let Value::Map(map_value) = key_value {
//         return Some(map_value);
//     }
//     None
// }
//

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
// // TODO we probably have this function in dpp
// fn check_protocol_version_bytes(version_bytes: &[u8]) -> bool {
//     if version_bytes.len() != 4 {
//         false
//     } else {
//         let version_set_bytes: [u8; 4] = version_bytes
//             .try_into()
//             .expect("slice with incorrect length");
//         let version = u32::from_be_bytes(version_set_bytes);
//         // todo despite the const this will be use as dynamic content
//         check_protocol_version(version)
//     }
// }
//
// const fn check_protocol_version(_version: u32) -> bool {
//     // Temporary disabled due protocol version is dynamic and goes from consensus params
//     true
// }
