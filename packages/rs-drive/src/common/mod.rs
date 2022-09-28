// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Common functions.
//!
//! This module defines general, commonly used functions in Drive.
//!

/// Encode module
pub mod encode;
/// Helpers module
pub mod helpers;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::option::Option::None;
use std::path::Path;

use byteorder::{BigEndian, WriteBytesExt};
use ciborium::value::Value;
use grovedb::TransactionArg;

use crate::contract::Contract;
use crate::drive::flags::StorageFlags;
use crate::drive::Drive;
use crate::error::structure::StructureError;
use crate::error::Error;

use dpp::data_contract::extra::DriveContractExt;

/// Serializes to CBOR and applies to Drive a JSON contract from the file system.
pub fn setup_contract(
    drive: &Drive,
    path: &str,
    contract_id: Option<[u8; 32]>,
    transaction: TransactionArg,
) -> Contract {
    let contract_cbor = json_document_to_cbor(path, Some(crate::drive::defaults::PROTOCOL_VERSION));
    let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, contract_id)
        .expect("contract should be deserialized");
    let contract_cbor =
        DriveContractExt::to_cbor(&contract).expect("contract should be serialized");

    drive
        .apply_contract_cbor(
            contract_cbor,
            contract_id,
            0f64,
            true,
            StorageFlags::default(),
            transaction,
        )
        .expect("contract should be applied");
    contract
}

/// Serializes to CBOR and applies to Drive a contract from hex string format.
pub fn setup_contract_from_hex(
    drive: &Drive,
    hex_string: String,
    transaction: TransactionArg,
) -> Contract {
    let contract_cbor = cbor_from_hex(hex_string);
    let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
        .expect("contract should be deserialized");
    drive
        .apply_contract_cbor(
            contract_cbor,
            None,
            0f64,
            true,
            StorageFlags::default(),
            transaction,
        )
        .expect("contract should be applied");
    contract
}

/// Reads a JSON file and converts it to CBOR.
pub fn json_document_to_cbor(path: impl AsRef<Path>, protocol_version: Option<u32>) -> Vec<u8> {
    let file = File::open(path).expect("file not found");
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader).expect("expected a valid json");
    value_to_cbor(json, protocol_version)
}

/// Serializes a JSON value to CBOR.
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

/// Serializes a hex string to CBOR.
pub fn cbor_from_hex(hex_string: String) -> Vec<u8> {
    hex::decode(hex_string).expect("Decoding failed")
}

/// Takes a file and returns the lines as a list of strings.
pub fn text_file_strings(path: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(path).expect("file not found");
    let reader = io::BufReader::new(file).lines();
    reader.into_iter().map(|a| a.unwrap()).collect()
}

/// Retrieves the value of a key from a CBOR map.
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

/// Converts a CBOR map to a BTree map.
pub fn cbor_map_to_btree_map(cbor_map: &[(Value, Value)]) -> BTreeMap<String, &Value> {
    cbor_map
        .iter()
        .filter_map(|(key, value)| key.as_text().map(|key| (key.to_string(), value)))
        .collect::<BTreeMap<String, &Value>>()
}

/// Converts the keys which are dynamic CBOR strings from a CBOR map to a BTree map.
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

/// Retrieves the value of a key from a CBOR map if it's an array.
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

/// Retrieves the value of a key from a CBOR map if it's an array of strings.
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

/// Retrieves the value of a key from a CBOR map if it's a map itself.
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
) -> Option<&'a str> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Text(string_value) = key_value {
        return Some(string_value);
    }
    None
}

/// Retrieves the value of a key from a B-tree map if it's a string.
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

/// Retrieves the value of a key from a CBOR map if it's a byte array.
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
                            Error::Structure(StructureError::ValueWrongType("expected u8 value"))
                        })?;
                        Ok(value_as_u8)
                    }
                    _ => Err(Error::Structure(StructureError::ValueWrongType(
                        "not an array of integers",
                    ))),
                })
                .collect::<Result<Vec<u8>, Error>>()
            {
                Ok(bytes) => Some(bytes),
                Err(_) => None,
            }
        }
        _ => None,
    }
}

/// Retrieves the value of a key from a CBOR map if it's a boolean.
pub fn cbor_inner_bool_value(document_type: &[(Value, Value)], key: &str) -> Option<bool> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Bool(bool_value) = key_value {
        return Some(*bool_value);
    }
    None
}

/// Retrieves the value of a key from a B-tree map if it's a boolean.
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

/// Retrieves the value of a key from a CBOR map if it's a u8 value.
pub fn cbor_inner_size_value(document_type: &[(Value, Value)], key: &str) -> Option<usize> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Integer(integer) = key_value {
        let value_as_usize: Result<usize, Error> = (*integer)
            .try_into()
            .map_err(|_| Error::Structure(StructureError::ValueWrongType("expected u8 value")));
        match value_as_usize {
            Ok(size) => Some(size),
            Err(_) => None,
        }
    } else {
        None
    }
}

/// Retrieves the value of a key from a B-tree map if it's a u8 value.
pub fn btree_map_inner_size_value(
    document_type: &BTreeMap<String, &Value>,
    key: &str,
) -> Option<usize> {
    let key_value = document_type.get(key)?;
    if let Value::Integer(integer) = key_value {
        let value_as_usize: Result<usize, Error> = (*integer)
            .try_into()
            .map_err(|_| Error::Structure(StructureError::ValueWrongType("expected u8 value")));
        match value_as_usize {
            Ok(size) => Some(size),
            Err(_) => None,
        }
    } else {
        None
    }
}

/// Retrieves the value of a key from a CBOR map if it's a boolean otherwise returns a default boolean.
pub fn cbor_inner_bool_value_with_default(
    document_type: &[(Value, Value)],
    key: &str,
    default: bool,
) -> bool {
    cbor_inner_bool_value(document_type, key).unwrap_or(default)
}

/// Takes a value (should be a system value) and returns it as a byte array if possible.
pub fn bytes_for_system_value(value: &Value) -> Result<Option<Vec<u8>>, Error> {
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
                        Error::Structure(StructureError::ValueWrongType("expected u8 value"))
                    })?;
                    Ok(Some(value_as_u8))
                }
                _ => Err(Error::Structure(StructureError::ValueWrongType(
                    "not an array of integers",
                ))),
            })
            .collect::<Result<Option<Vec<u8>>, Error>>(),
        _ => Err(Error::Structure(StructureError::ValueWrongType(
            "system value is incorrect type",
        ))),
    }
}

/// Takes a B-tree map and a key and returns the corresponding value (should be a system value)
/// as a byte array if possible.
pub fn bytes_for_system_value_from_tree_map(
    document: &BTreeMap<String, Value>,
    key: &str,
) -> Result<Option<Vec<u8>>, Error> {
    let value = document.get(key);
    if let Some(value) = value {
        bytes_for_system_value(value)
    } else {
        Ok(None)
    }
}

/// Takes a B-tree map, a key, and a default bool values and returns the corresponding
/// value (should be a system value) from the key if it's a boolean, otherwise returns the default.
pub fn bool_for_system_value_from_tree_map(
    document: &BTreeMap<String, Value>,
    key: &str,
    default: bool,
) -> Result<bool, Error> {
    let value = document.get(key);
    if let Some(value) = value {
        if let Value::Bool(bool_value) = value {
            Ok(*bool_value)
        } else {
            Err(Error::Structure(StructureError::ValueWrongType(
                "value is expected to be a boolean",
            )))
        }
    } else {
        Ok(default)
    }
}

/// Retrieves the value of a key from a CBOR map if it's a u64.
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

/// Retrieves the value of a key from a CBOR map if it's a u32.
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

/// Retrieves the value of a key from a CBOR map if it's a u16.
pub(crate) fn cbor_inner_u16_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<u16> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Integer(integer_value) = key_value {
        return Some(i128::from(*integer_value) as u16);
    }
    None
}

/// Retrieves the value of a key from a CBOR map if it's a u8.
pub(crate) fn cbor_inner_u8_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<u8> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Integer(integer_value) = key_value {
        return Some(i128::from(*integer_value) as u8);
    }
    None
}
