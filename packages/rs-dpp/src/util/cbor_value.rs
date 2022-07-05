use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use anyhow::anyhow;
use ciborium::value::Value as CborValue;

use serde_json::{Map, Value as JsonValue};

use crate::common::bytes_for_system_value_from_tree_map;
use crate::identifier::Identifier;
use crate::util::json_value::ReplaceWith;
use crate::util::string_encoding::Encoding;
use crate::ProtocolError;

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

pub trait CborBTreeMapHelper {
    fn get_identifier(&self, key: &str) -> Result<[u8; 32], ProtocolError>;
    fn get_string(&self, key: &str) -> Result<String, ProtocolError>;
    fn get_u32(&self, key: &str) -> Result<u32, ProtocolError>;
    fn get_i64(&self, key: &str) -> Result<i64, ProtocolError>;
}

impl CborBTreeMapHelper for BTreeMap<String, CborValue> {
    fn get_identifier(&self, key: &str) -> Result<[u8; 32], ProtocolError> {
        bytes_for_system_value_from_tree_map(self, key)?
            .ok_or_else(|| {
                ProtocolError::DecodingError(format!("unable to get {key}"))
            })?
            .try_into()
            .map_err(|_| {
                ProtocolError::DecodingError(format!("{key} must be 32 bytes"))
            })
    }

    fn get_string(&self, key: &str) -> Result<String, ProtocolError> {
        Ok(self
            .get(key)
            .ok_or_else(|| {
                ProtocolError::DecodingError(format!("unable to get {key}"))
            })?
            .as_text()
            .ok_or_else(|| {
                ProtocolError::DecodingError(format!("expect {key} to be a string"))
            })?
            .to_string())
    }

    fn get_u32(&self, key: &str) -> Result<u32, ProtocolError> {
        Ok(i128::from(
            self.get(key)
                .ok_or_else(|| {
                    ProtocolError::DecodingError(format!("unable to get {key}"))
                })?
                .as_integer()
                .ok_or_else(|| {
                    ProtocolError::DecodingError(format!(
                        "expect {key} to be an integer"
                    ))
                })?,
        ) as u32)
    }

    fn get_i64(&self, key: &str) -> Result<i64, ProtocolError> {
        self
            .get(key)
            .ok_or_else(|| {
                ProtocolError::DecodingError(format!("unable to get property {key}"))
            })?
            .as_integer()
            .ok_or_else(|| {
                ProtocolError::DecodingError(format!("{key} must be an integer"))
            })?
            .try_into()
            .map_err(|_| {
                ProtocolError::DecodingError(format!("{key} must be a 64 int"))
            })
    }
}

pub trait CborMapExtension {
    fn as_u16(&self, key: &str, error_message: &str) -> Result<u16, ProtocolError>;
    fn as_u8(&self, key: &str, error_message: &str) -> Result<u8, ProtocolError>;
    fn as_bool(&self, key: &str, error_message: &str) -> Result<bool, ProtocolError>;
    fn as_bytes(&self, key: &str, error_message: &str) -> Result<Vec<u8>, ProtocolError>;
    fn as_string(&self, key: &str, error_message: &str) -> Result<String, ProtocolError>;
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
}

fn recursively_sort_canonical_cbor_map(cbor_map: &mut [(CborValue, CborValue)]) {
    for (_, value) in cbor_map.iter_mut() {
        if let CborValue::Map(map) = value {
            recursively_sort_canonical_cbor_map(map)
        }
    }

    cbor_map.sort_by(|a, b| {
        // We now for sure that the keys are always text, since `insert()`
        // methods accepts only types that can be converted into a string
        let key_a = a.0.as_text().unwrap().as_bytes();
        let key_b = b.0.as_text().unwrap().as_bytes();

        let len_comparison = key_a.len().cmp(&key_b.len());

        match len_comparison {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => key_a.cmp(key_b),
            Ordering::Greater => Ordering::Greater,
        }
    });
}

pub fn replace_binary(to_replace: &mut CborValue, with: ReplaceWith) -> Result<(), anyhow::Error> {
    let mut cbor_value = CborValue::Null;
    std::mem::swap(to_replace, &mut cbor_value);
    match with {
        ReplaceWith::Base58 => {
            let data_bytes = cbor_value
                .as_bytes()
                .ok_or_else(|| anyhow!("expect value to be bytes"))?;
            *to_replace = CborValue::Text(bs58::encode(data_bytes).into_string());
        }
        ReplaceWith::Base64 => {
            let data_bytes = cbor_value
                .as_bytes()
                .ok_or_else(|| anyhow!("expect value to be bytes"))?;
            *to_replace = CborValue::Text(base64::encode(data_bytes));
        }
        ReplaceWith::Bytes => {
            let data_string = String::from(
                cbor_value
                    .as_text()
                    .ok_or_else(|| anyhow!("expect value to be string"))?,
            );
            let identifier = Identifier::from_string(&data_string, Encoding::Base58)?.to_buffer();
            *to_replace = CborValue::Bytes(identifier.to_vec());
        }
    }
    Ok(())
}

#[derive(Default, Clone, Debug)]
pub struct CborCanonicalMap {
    inner: Vec<(CborValue, CborValue)>,
}

impl CborCanonicalMap {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn from_vector(vec: Vec<(CborValue, CborValue)>) -> Self {
        let mut map = Self::new();
        map.inner = vec;
        map
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<CborValue>) {
        self.inner.push((CborValue::Text(key.into()), value.into()));
    }

    pub fn remove(&mut self, key_to_remove: impl Into<CborValue>) {
        let key_to_compare: CborValue = key_to_remove.into();
        if let Some(index) = self
            .inner
            .iter()
            .position(|(key, _)| key == &key_to_compare)
        {
            self.inner.remove(index);
        }
    }

    pub fn get_mut(&mut self, key: &CborValue) -> Option<&mut CborValue> {
        if let Some(index) = self.inner.iter().position(|(el_key, _)| el_key == key) {
            Some(&mut self.inner.get_mut(index)?.1)
        } else {
            None
        }
    }

    pub fn replace_values<I, C>(&mut self, keys: I, with: ReplaceWith)
    where
        I: IntoIterator<Item = C>,
        C: Into<CborValue>,
    {
        for key in keys.into_iter() {
            self.replace_value(key, with);
        }
    }

    pub fn replace_value(&mut self, key: impl Into<CborValue>, with: ReplaceWith) -> Option<()> {
        let k = key.into();
        let cbor_value = self.get_mut(&k)?;

        let replace_with = match with {
            ReplaceWith::Base58 => {
                let data_bytes = cbor_value.as_bytes()?;
                CborValue::Text(bs58::encode(data_bytes).into_string())
            }
            ReplaceWith::Base64 => {
                let data_bytes = cbor_value.as_bytes()?;
                CborValue::Text(base64::encode(data_bytes))
            }
            ReplaceWith::Bytes => {
                let data_string = String::from(cbor_value.as_text()?);
                let identifier = Identifier::from_string(&data_string, Encoding::Base58)
                    .ok()?
                    .to_buffer();
                CborValue::Bytes(identifier.to_vec())
            }
        };

        self.set(&k, replace_with);

        Some(())
    }

    pub fn set(&mut self, key: &CborValue, replace_with: CborValue) -> Option<()> {
        if let Some(index) = self.inner.iter().position(|(el_key, _)| el_key == key) {
            if let Some(key_value) = self.inner.get_mut(index) {
                key_value.1 = replace_with;
                Some(())
            } else {
                None
            }
        } else {
            None
        }
    }

    // pub fn replace_values<'a>(
    //     &mut self,
    //     paths: impl IntoIterator<Item = &'a str>,
    //     with: ReplaceWith,
    // ) -> Result<(), anyhow::Error> {
    //     for raw_path in paths {
    //         let mut to_replace = get_value_mut(raw_path, self);
    //         match to_replace {
    //             Some(ref mut v) => {
    //                 replace_identifier(v, with).map_err(|err| {
    //                     anyhow!(
    //                         "unable replace the {:?} with {:?}: '{}'",
    //                         raw_path,
    //                         with,
    //                         err
    //                     )
    //                 })?;
    //             }
    //             None => {
    //                 trace!("path '{}' is not found, replacing to {:?} ", raw_path, with)
    //             }
    //         }
    //     }
    //     Ok(())
    // }

    /// From the CBOR RFC on how to sort the keys:
    /// *  If two keys have different lengths, the shorter one sorts
    ///    earlier;
    ///
    /// *  If two keys have the same length, the one with the lower value
    ///    in (byte-wise) lexical order sorts earlier.
    ///
    /// https://datatracker.ietf.org/doc/html/rfc7049#section-3.9
    pub fn sort_canonical(&mut self) {
        recursively_sort_canonical_cbor_map(&mut self.inner)
    }

    pub fn to_bytes(mut self) -> Result<Vec<u8>, ciborium::ser::Error<std::io::Error>> {
        self.sort_canonical();

        let mut bytes = Vec::<u8>::new();

        let map = CborValue::Map(self.inner);

        ciborium::ser::into_writer(&map, &mut bytes)?;

        Ok(bytes)
    }

    pub fn to_value_unsorted(&self) -> CborValue {
        CborValue::Map(self.inner.clone())
    }

    pub fn to_value_sorted(mut self) -> CborValue {
        self.sort_canonical();

        CborValue::Map(self.inner)
    }

    pub fn to_value_clone(&mut self) -> CborValue {
        self.sort_canonical();

        CborValue::Map(self.inner.clone())
    }
}

impl TryFrom<CborValue> for CborCanonicalMap {
    type Error = ProtocolError;

    fn try_from(value: CborValue) -> Result<Self, Self::Error> {
        if let CborValue::Map(map) = value {
            Ok(Self::from_vector(map))
        } else {
            Err(ProtocolError::ParsingError(
                "Expected map to be a map".into(),
            ))
        }
    }
}

impl From<Vec<(CborValue, CborValue)>> for CborCanonicalMap {
    fn from(vec: Vec<(CborValue, CborValue)>) -> Self {
        Self::from_vector(vec)
    }
}

impl From<&Vec<(CborValue, CborValue)>> for CborCanonicalMap {
    fn from(vec: &Vec<(CborValue, CborValue)>) -> Self {
        Self::from_vector(vec.clone())
    }
}

// pub fn json_value_to_cbor_value(&json: JsonValueValue) -> CborValue {
//
// }

// impl From<&BTreeMap<String, JsonValueValue>> for CborCanonicalMap {
//     fn from(map: &BTreeMap<String, JsonValueValue>) -> Self {
//         let vec = map.iter().map(|(key, value)| {
//             (key.into(), value.into())
//         }).collect::<Vec<(CborValue, CborValue)>>();
//
//         Self::from(vec)
//     }
// }

impl<T> From<&BTreeMap<String, T>> for CborCanonicalMap
where
    T: Into<CborValue> + Clone,
{
    fn from(map: &BTreeMap<String, T>) -> Self {
        let vec = map
            .iter()
            .map(|(key, value)| (key.clone().into(), value.clone().into()))
            .collect::<Vec<(CborValue, CborValue)>>();

        Self::from(vec)
    }
}

pub fn cbor_value_to_json_value(cbor: &CborValue) -> Result<serde_json::Value, ProtocolError> {
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
                .collect::<Result<Vec<JsonValue>, ProtocolError>>()?,
        )),
        CborValue::Map(map) => cbor_map_to_json_map(map),
        _ => Err(ProtocolError::DecodingError(String::from(
            "Can't convert CBOR to JSON: unknown type",
        ))),
    }
}

pub fn cbor_map_to_json_map(
    cbor_map: &[(CborValue, CborValue)],
) -> Result<serde_json::Value, ProtocolError> {
    let mut json_vec = cbor_map
        .iter()
        .map(|(key, value)| {
            Ok((
                key.as_text()
                    .ok_or_else(|| {
                        ProtocolError::DecodingError(String::from("Expect key to be a string"))
                    })?
                    .to_string(),
                cbor_value_to_json_value(value)?,
            ))
        })
        .collect::<Result<Vec<(String, JsonValue)>, ProtocolError>>()?;

    let mut json_map = Map::new();

    for (key, value) in json_vec.drain(..) {
        json_map.insert(key, value);
    }

    Ok(serde_json::Value::Object(json_map))
}
