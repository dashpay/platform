use std::convert::TryInto;

use anyhow::anyhow;
use ciborium::value::Value as CborValue;
use serde_json::{Map, Value as JsonValue};

use crate::ProtocolError;
mod convert;
pub use convert::FieldType;

mod value;
pub use value::*;

mod canonical;
pub use canonical::*;

mod map;
pub use map::*;

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

// TODO: the issue with stack overflow should be address through re-implementation of the algorithm
pub fn cbor_value_to_json_value(cbor: &CborValue) -> Result<serde_json::Value, anyhow::Error> {
    match cbor {
        CborValue::Integer(num) => Ok(JsonValue::from(i128::from(*num) as i64)),
        CborValue::Bytes(bytes) => Ok(JsonValue::Array(
            bytes.iter().map(|byte| JsonValue::from(*byte)).collect(),
        )),
        CborValue::Float(float) => Ok(JsonValue::from(*float)),
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
            bytes.into_iter().map(JsonValue::from).collect(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::value::Value as CborValue;
    use serde_json::json;

    // --- get_key_from_cbor_map ---

    #[test]
    fn get_key_from_cbor_map_found() {
        let map = vec![
            (
                CborValue::Text("name".to_string()),
                CborValue::Text("Alice".to_string()),
            ),
            (
                CborValue::Text("age".to_string()),
                CborValue::Integer(30.into()),
            ),
        ];
        let result = get_key_from_cbor_map(&map, "name");
        assert_eq!(result, Some(&CborValue::Text("Alice".to_string())));
    }

    #[test]
    fn get_key_from_cbor_map_not_found() {
        let map = vec![(
            CborValue::Text("name".to_string()),
            CborValue::Text("Alice".to_string()),
        )];
        let result = get_key_from_cbor_map(&map, "missing");
        assert!(result.is_none());
    }

    #[test]
    fn get_key_from_cbor_map_skips_non_text_keys() {
        let map = vec![
            (CborValue::Integer(1.into()), CborValue::Bool(true)),
            (CborValue::Text("key".to_string()), CborValue::Bool(false)),
        ];
        let result = get_key_from_cbor_map(&map, "key");
        assert_eq!(result, Some(&CborValue::Bool(false)));
    }

    // --- CborMapExtension for &Vec<(CborValue, CborValue)> ---

    fn make_cbor_map(pairs: Vec<(&str, CborValue)>) -> Vec<(CborValue, CborValue)> {
        pairs
            .into_iter()
            .map(|(k, v)| (CborValue::Text(k.to_string()), v))
            .collect()
    }

    #[test]
    fn cbor_map_extension_as_u16() {
        let map = make_cbor_map(vec![("val", CborValue::Integer(1234.into()))]);
        let result = (&map).as_u16("val", "err");
        assert_eq!(result.unwrap(), 1234);
    }

    #[test]
    fn cbor_map_extension_as_u16_missing() {
        let map = make_cbor_map(vec![]);
        let result = (&map).as_u16("val", "missing");
        assert!(result.is_err());
    }

    #[test]
    fn cbor_map_extension_as_u16_wrong_type() {
        let map = make_cbor_map(vec![("val", CborValue::Text("hello".to_string()))]);
        let result = (&map).as_u16("val", "err");
        assert!(result.is_err());
    }

    #[test]
    fn cbor_map_extension_as_u8() {
        let map = make_cbor_map(vec![("val", CborValue::Integer(255.into()))]);
        let result = (&map).as_u8("val", "err");
        assert_eq!(result.unwrap(), 255);
    }

    #[test]
    fn cbor_map_extension_as_bool() {
        let map = make_cbor_map(vec![("flag", CborValue::Bool(true))]);
        let result = (&map).as_bool("flag", "err");
        assert!(result.unwrap());
    }

    #[test]
    fn cbor_map_extension_as_bool_missing() {
        let map = make_cbor_map(vec![]);
        let result = (&map).as_bool("flag", "missing");
        assert!(result.is_err());
    }

    #[test]
    fn cbor_map_extension_as_bool_wrong_type() {
        let map = make_cbor_map(vec![("flag", CborValue::Integer(1.into()))]);
        let result = (&map).as_bool("flag", "err");
        assert!(result.is_err());
    }

    #[test]
    fn cbor_map_extension_as_bytes_from_bytes() {
        let data = vec![1u8, 2, 3, 4];
        let map = make_cbor_map(vec![("data", CborValue::Bytes(data.clone()))]);
        let result = (&map).as_bytes("data", "err");
        assert_eq!(result.unwrap(), data);
    }

    #[test]
    fn cbor_map_extension_as_bytes_from_array() {
        let array = vec![CborValue::Integer(10.into()), CborValue::Integer(20.into())];
        let map = make_cbor_map(vec![("data", CborValue::Array(array))]);
        let result = (&map).as_bytes("data", "err");
        assert_eq!(result.unwrap(), vec![10u8, 20]);
    }

    #[test]
    fn cbor_map_extension_as_bytes_wrong_type() {
        let map = make_cbor_map(vec![("data", CborValue::Bool(true))]);
        let result = (&map).as_bytes("data", "err");
        assert!(result.is_err());
    }

    #[test]
    fn cbor_map_extension_as_string() {
        let map = make_cbor_map(vec![("s", CborValue::Text("hello".to_string()))]);
        let result = (&map).as_string("s", "err");
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn cbor_map_extension_as_string_wrong_type() {
        let map = make_cbor_map(vec![("s", CborValue::Integer(1.into()))]);
        let result = (&map).as_string("s", "err");
        assert!(result.is_err());
    }

    #[test]
    fn cbor_map_extension_as_u64() {
        let map = make_cbor_map(vec![("n", CborValue::Integer(999999.into()))]);
        let result = (&map).as_u64("n", "err");
        assert_eq!(result.unwrap(), 999999);
    }

    // --- cbor_value_to_json_value ---

    #[test]
    fn cbor_integer_to_json() {
        let result = cbor_value_to_json_value(&CborValue::Integer(42.into())).unwrap();
        assert_eq!(result, json!(42));
    }

    #[test]
    fn cbor_text_to_json() {
        let result = cbor_value_to_json_value(&CborValue::Text("hello".to_string())).unwrap();
        assert_eq!(result, json!("hello"));
    }

    #[test]
    fn cbor_bool_to_json() {
        let result = cbor_value_to_json_value(&CborValue::Bool(true)).unwrap();
        assert_eq!(result, json!(true));
    }

    #[test]
    fn cbor_null_to_json() {
        let result = cbor_value_to_json_value(&CborValue::Null).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn cbor_float_to_json() {
        let result = cbor_value_to_json_value(&CborValue::Float(3.14)).unwrap();
        assert_eq!(result, json!(3.14));
    }

    #[test]
    fn cbor_bytes_to_json_array() {
        let result = cbor_value_to_json_value(&CborValue::Bytes(vec![1, 2, 3])).unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn cbor_array_to_json_array() {
        let cbor = CborValue::Array(vec![
            CborValue::Integer(1.into()),
            CborValue::Text("two".to_string()),
        ]);
        let result = cbor_value_to_json_value(&cbor).unwrap();
        assert_eq!(result, json!([1, "two"]));
    }

    #[test]
    fn cbor_map_to_json_object() {
        let cbor = CborValue::Map(vec![(
            CborValue::Text("key".to_string()),
            CborValue::Integer(10.into()),
        )]);
        let result = cbor_value_to_json_value(&cbor).unwrap();
        assert_eq!(result, json!({"key": 10}));
    }

    // --- cbor_value_into_json_value (owned) ---

    #[test]
    fn cbor_into_json_integer() {
        let result = cbor_value_into_json_value(CborValue::Integer(99.into())).unwrap();
        assert_eq!(result, json!(99));
    }

    #[test]
    fn cbor_into_json_text() {
        let result = cbor_value_into_json_value(CborValue::Text("world".to_string())).unwrap();
        assert_eq!(result, json!("world"));
    }

    #[test]
    fn cbor_into_json_null() {
        let result = cbor_value_into_json_value(CborValue::Null).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn cbor_into_json_nested_array() {
        let cbor = CborValue::Array(vec![CborValue::Array(vec![CborValue::Integer(1.into())])]);
        let result = cbor_value_into_json_value(cbor).unwrap();
        assert_eq!(result, json!([[1]]));
    }

    // --- cbor_map_to_json_map ---

    #[test]
    fn test_cbor_map_to_json_map_valid() {
        let map = vec![
            (
                CborValue::Text("a".to_string()),
                CborValue::Integer(1.into()),
            ),
            (CborValue::Text("b".to_string()), CborValue::Bool(false)),
        ];
        let result = cbor_map_to_json_map(&map).unwrap();
        assert_eq!(result, json!({"a": 1, "b": false}));
    }

    #[test]
    fn test_cbor_map_to_json_map_non_string_key() {
        let map = vec![(CborValue::Integer(1.into()), CborValue::Bool(true))];
        let result = cbor_map_to_json_map(&map);
        assert!(result.is_err());
    }

    // --- cbor_map_into_json_map (owned) ---

    #[test]
    fn test_cbor_map_into_json_map_valid() {
        let map = vec![(
            CborValue::Text("x".to_string()),
            CborValue::Text("y".to_string()),
        )];
        let result = cbor_map_into_json_map(map).unwrap();
        assert_eq!(result, json!({"x": "y"}));
    }

    #[test]
    fn test_cbor_map_into_json_map_non_string_key() {
        let map = vec![(CborValue::Integer(1.into()), CborValue::Bool(true))];
        let result = cbor_map_into_json_map(map);
        assert!(result.is_err());
    }
}
