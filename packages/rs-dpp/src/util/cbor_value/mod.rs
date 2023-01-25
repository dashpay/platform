use std::convert::TryInto;

use anyhow::anyhow;
use ciborium::value::Value as CborValue;
use serde_json::{Map, Value as JsonValue};

use crate::ProtocolError;
mod convert;
pub use convert::FieldType;

mod cbor_value;
pub use cbor_value::*;

mod canonical;
pub use canonical::*;

mod cbor_map;
pub use cbor_map::*;

pub trait ValuesCollection {
    type Key;
    type Value;

    fn get_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Value>;
    fn get(&self, key: &Self::Key) -> Option<&Self::Value>;
    fn remove(&mut self, key_to_remove: impl Into<Self::Key>) -> Option<Self::Value>;
}

pub trait ReplacePaths: ValuesCollection {
    type Value;

    fn replace_paths<I, C>(&mut self, paths: I, from: FieldType, to: FieldType)
    where
        I: IntoIterator<Item = C>,
        C: AsRef<str>;

    fn replace_path(&mut self, path: &str, from: FieldType, to: FieldType) -> Option<()>;
    fn get_path_mut(&mut self, path: &str) -> Option<&mut <Self as ReplacePaths>::Value>;
}

pub fn get_key_from_cbor_map<'a>(
    cbor_map: &'a [(CborValue, CborValue)],
    key: &'a str,
) -> Option<&'a CborValue> {
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

impl CborMapExtension for &Vec<(CborValue, CborValue)> {
    fn as_u16(&self, key: &str, error_message: &str) -> Result<u16, ProtocolError> {
        let key_value = get_key_from_cbor_map(self, key)
            .ok_or_else(|| ProtocolError::DecodingError(String::from(error_message)))?;
        if let CborValue::Integer(integer_value) = key_value {
            return Ok(i128::from(*integer_value) as u16);
        }
        Err(ProtocolError::DecodingError(String::from(error_message)))
    }

    fn as_u8(&self, key: &str, error_message: &str) -> Result<u8, ProtocolError> {
        let key_value = get_key_from_cbor_map(self, key)
            .ok_or_else(|| ProtocolError::DecodingError(String::from(error_message)))?;
        if let CborValue::Integer(integer_value) = key_value {
            return Ok(i128::from(*integer_value) as u8);
        }
        Err(ProtocolError::DecodingError(String::from(error_message)))
    }

    fn as_bool(&self, key: &str, error_message: &str) -> Result<bool, ProtocolError> {
        let key_value = get_key_from_cbor_map(self, key)
            .ok_or_else(|| ProtocolError::DecodingError(String::from(error_message)))?;
        if let CborValue::Bool(bool_value) = key_value {
            return Ok(*bool_value);
        }
        Err(ProtocolError::DecodingError(String::from(error_message)))
    }

    fn as_bytes(&self, key: &str, error_message: &str) -> Result<Vec<u8>, ProtocolError> {
        let key_value = get_key_from_cbor_map(self, key)
            .ok_or_else(|| ProtocolError::DecodingError(String::from(error_message)))?;
        match key_value {
            CborValue::Bytes(bytes) => Ok(bytes.clone()),
            CborValue::Array(array) => array
                .iter()
                .map(|byte| match byte {
                    CborValue::Integer(int) => {
                        let value_as_u8: u8 = (*int).try_into().map_err(|_| {
                            ProtocolError::DecodingError(String::from("expected u8 value"))
                        })?;
                        Ok(value_as_u8)
                    }
                    _ => Err(ProtocolError::DecodingError(String::from(
                        "not an array of integers",
                    ))),
                })
                .collect::<Result<Vec<u8>, ProtocolError>>(),
            _ => Err(ProtocolError::DecodingError(String::from(error_message))),
        }
    }

    fn as_string(&self, key: &str, error_message: &str) -> Result<String, ProtocolError> {
        let key_value = get_key_from_cbor_map(self, key)
            .ok_or_else(|| ProtocolError::DecodingError(String::from(error_message)))?;
        if let CborValue::Text(string_value) = key_value {
            return Ok(string_value.clone());
        }
        Err(ProtocolError::DecodingError(String::from(error_message)))
    }

    fn as_u64(&self, key: &str, error_message: &str) -> Result<u64, ProtocolError> {
        let key_value = get_key_from_cbor_map(self, key)
            .ok_or_else(|| ProtocolError::DecodingError(String::from(error_message)))?;
        if let CborValue::Integer(integer_value) = key_value {
            return Ok(i128::from(*integer_value) as u64);
        }
        Err(ProtocolError::DecodingError(String::from(error_message)))
    }
}

// TODO: the issue with stack overflow should be address through re-implemtation of the algorithm
pub fn cbor_value_to_json_value(cbor: &CborValue) -> Result<serde_json::Value, anyhow::Error> {
    match cbor {
        CborValue::Integer(num) => Ok(JsonValue::from(i128::from(*num) as i64)),
        CborValue::Bytes(bytes) => Ok(JsonValue::Array(
            bytes.iter().map(|byte| JsonValue::from(*byte)).collect(),
        )),
        CborValue::Float(float) => Ok(JsonValue::from(*float as f64)),
        CborValue::Text(text) => Ok(JsonValue::from(text.clone())),
        CborValue::Bool(boolean) => Ok(JsonValue::from(*boolean)),
        CborValue::Null => Ok(JsonValue::Null),
        CborValue::Array(arr) => Ok(JsonValue::Array(
            arr.iter()
                .map(cbor_value_to_json_value)
                .collect::<Result<Vec<JsonValue>, anyhow::Error>>()?,
        )),
        CborValue::Map(map) => cbor_map_to_json_map(map),
        _ => Err(anyhow!("Can't convert CBOR to JSON: unknown type")),
    }
}

pub fn cbor_value_into_json_value(cbor: CborValue) -> Result<serde_json::Value, anyhow::Error> {
    match cbor {
        CborValue::Integer(num) => Ok(JsonValue::from(i128::from(num) as i64)),
        CborValue::Bytes(bytes) => Ok(JsonValue::Array(
            bytes
                .into_iter()
                .map(|byte| JsonValue::from(byte))
                .collect(),
        )),
        CborValue::Float(float) => Ok(JsonValue::from(float)),
        CborValue::Text(text) => Ok(JsonValue::from(text)),
        CborValue::Bool(boolean) => Ok(JsonValue::from(boolean)),
        CborValue::Null => Ok(JsonValue::Null),
        CborValue::Array(arr) => Ok(JsonValue::Array(
            arr.into_iter()
                .map(cbor_value_into_json_value)
                .collect::<Result<Vec<JsonValue>, anyhow::Error>>()?,
        )),
        CborValue::Map(map) => cbor_map_into_json_map(map),
        _ => Err(anyhow!("Can't convert CBOR to JSON: unknown type")),
    }
}

pub fn cbor_map_to_json_map(
    cbor_map: &[(CborValue, CborValue)],
) -> Result<serde_json::Value, anyhow::Error> {
    let mut json_vec = cbor_map
        .iter()
        .map(|(key, value)| {
            Ok((
                key.as_text()
                    .ok_or_else(|| anyhow!("Expect key to be a string"))?
                    .to_string(),
                cbor_value_to_json_value(value)?,
            ))
        })
        .collect::<Result<Vec<(String, JsonValue)>, anyhow::Error>>()?;

    let mut json_map = Map::new();

    for (key, value) in json_vec.drain(..) {
        json_map.insert(key, value);
    }

    Ok(serde_json::Value::Object(json_map))
}

pub fn cbor_map_into_json_map(
    cbor_map: Vec<(CborValue, CborValue)>,
) -> Result<serde_json::Value, anyhow::Error> {
    let mut json_vec = cbor_map
        .into_iter()
        .map(|(key, value)| {
            Ok((
                key.into_text()
                    .map_err(|_| anyhow!("Expect key to be a string"))?,
                cbor_value_into_json_value(value)?,
            ))
        })
        .collect::<Result<Vec<(String, JsonValue)>, anyhow::Error>>()?;

    let mut json_map = Map::new();

    for (key, value) in json_vec.drain(..) {
        json_map.insert(key, value);
    }

    Ok(serde_json::Value::Object(json_map))
}
