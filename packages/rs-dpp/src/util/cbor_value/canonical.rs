use std::{
    cmp::Ordering,
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
};

use anyhow::anyhow;
use ciborium::value::Value as CborValue;
use serde::Serialize;

use crate::{
    prelude::Identifier,
    util::{json_value::ReplaceWith, string_encoding::Encoding},
    ProtocolError,
};

use super::{
    convert::convert_to, get_from_cbor_map, to_path_of_cbors, FieldType, ReplacePaths,
    ValuesCollection,
};

#[derive(Default, Clone, Debug)]
pub struct CborCanonicalMap {
    inner: Vec<(CborValue, CborValue)>,
}

impl CborCanonicalMap {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn from_serializable<T>(value: &T) -> Result<Self, ProtocolError>
    where
        T: Serialize,
    {
        let cbor = ciborium::value::Value::serialized(&value)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
        CborCanonicalMap::try_from(cbor).map_err(|e| ProtocolError::EncodingError(e.to_string()))
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

    pub fn replace_paths<I, C>(&mut self, paths: I, from: FieldType, to: FieldType)
    where
        I: IntoIterator<Item = C>,
        C: AsRef<str>,
    {
        for path in paths.into_iter() {
            self.replace_path(path.as_ref(), from, to);
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

    pub fn replace_path(&mut self, path: &str, from: FieldType, to: FieldType) -> Option<()> {
        let cbor_value = self.get_path_mut(path)?;
        let replace_with = convert_to(cbor_value, from, to)?;

        *cbor_value = replace_with;

        Some(())
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

        ciborium::ser::SerializerOptions::default()
            .serialize_null_as_undefined(true)
            .into_writer(&map, &mut bytes)?;

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

impl ValuesCollection for CborCanonicalMap {
    type Key = CborValue;
    type Value = CborValue;

    fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
        if let Some(index) = self.inner.iter().position(|(el_key, _)| el_key == key) {
            Some(&self.inner.get(index)?.1)
        } else {
            None
        }
    }

    fn get_mut(&mut self, key: &CborValue) -> Option<&mut CborValue> {
        if let Some(index) = self.inner.iter().position(|(el_key, _)| el_key == key) {
            Some(&mut self.inner.get_mut(index)?.1)
        } else {
            None
        }
    }

    fn remove(&mut self, key_to_remove: impl Into<CborValue>) -> Option<Self::Value> {
        let key_to_compare: CborValue = key_to_remove.into();
        if let Some(index) = self
            .inner
            .iter()
            .position(|(key, _)| key == &key_to_compare)
        {
            let (_, v) = self.inner.remove(index);
            Some(v)
        } else {
            None
        }
    }
}

impl ReplacePaths for CborCanonicalMap {
    type Value = CborValue;

    fn replace_paths<I, C>(&mut self, paths: I, from: FieldType, to: FieldType)
    where
        I: IntoIterator<Item = C>,
        C: AsRef<str>,
    {
        for path in paths.into_iter() {
            self.replace_path(path.as_ref(), from, to);
        }
    }

    fn replace_path(&mut self, path: &str, from: FieldType, to: FieldType) -> Option<()> {
        let cbor_value = self.get_path_mut(path)?;
        let replace_with = convert_to(cbor_value, from, to)?;

        *cbor_value = replace_with;

        Some(())
    }

    fn get_path_mut(&mut self, path: &str) -> Option<&mut CborValue> {
        let cbor_path = to_path_of_cbors(path).ok()?;
        if cbor_path.is_empty() {
            return None;
        }
        if cbor_path.len() == 1 {
            return self.get_mut(&cbor_path[0]);
        }

        let mut current_level: &mut CborValue = self.get_mut(&cbor_path[0])?;
        for step in cbor_path.iter().skip(1) {
            match current_level {
                CborValue::Map(ref mut cbor_map) => {
                    current_level = get_from_cbor_map(cbor_map, step)?
                }
                CborValue::Array(ref mut cbor_array) => {
                    if let Some(idx) = step.as_integer() {
                        let id: usize = idx.try_into().ok()?;
                        current_level = cbor_array.get_mut(id)?
                    } else {
                        return None;
                    }
                }
                _ => {
                    // do nothing if it's not a container type
                }
            }
        }
        Some(current_level)
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

fn recursively_sort_canonical_cbor_map(cbor_map: &mut [(CborValue, CborValue)]) {
    for (_, value) in cbor_map.iter_mut() {
        if let CborValue::Map(map) = value {
            recursively_sort_canonical_cbor_map(map)
        }
        if let CborValue::Array(array) = value {
            for item in array.iter_mut() {
                if let CborValue::Map(map) = item {
                    recursively_sort_canonical_cbor_map(map)
                }
            }
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

//todo: explain why this returns an option?
pub fn value_to_bytes(value: &CborValue) -> Result<Option<Vec<u8>>, ProtocolError> {
    match value {
        CborValue::Bytes(bytes) => Ok(Some(bytes.clone())),
        CborValue::Text(text) => match bs58::decode(text).into_vec() {
            Ok(data) => Ok(Some(data)),
            Err(_) => Ok(None),
        },
        CborValue::Array(array) => array
            .iter()
            .map(|byte| match byte {
                CborValue::Integer(int) => {
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

pub fn value_to_hash(value: &CborValue) -> Result<[u8; 32], ProtocolError> {
    match value {
        CborValue::Bytes(bytes) => bytes
            .clone()
            .try_into()
            .map_err(|_| ProtocolError::DecodingError("expected 32 bytes".to_string())),
        CborValue::Text(text) => match bs58::decode(text).into_vec() {
            Ok(bytes) => bytes
                .try_into()
                .map_err(|_| ProtocolError::DecodingError("expected 32 bytes".to_string())),
            Err(_) => Err(ProtocolError::DecodingError(
                "expected 32 bytes".to_string(),
            )),
        },
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
            .collect::<Result<Vec<u8>, ProtocolError>>()?
            .try_into()
            .map_err(|_| ProtocolError::DecodingError("expected 32 bytes".to_string())),
        _ => Err(ProtocolError::DecodingError(String::from(
            "system value is incorrect type",
        ))),
    }
}

#[cfg(test)]
mod test {
    use std::convert::TryInto;

    use crate::util::cbor_value::ReplacePaths;

    use super::{CborValue, FieldType};
    use ciborium::cbor;
    use serde::{Deserialize, Serialize};

    use super::CborCanonicalMap;

    #[test]
    fn should_get_path_to_property_from_cbor() {
        let cbor_value = cbor!( {
            "alpha"  =>  {
                "bravo" =>  "bravo_value",
            }
        })
        .expect("valid cbor");
        let mut canonical: CborCanonicalMap = cbor_value.try_into().expect("valid canonical");
        let result = canonical.get_path_mut("alpha.bravo").expect("bravo value");
        assert_eq!(&mut CborValue::Text(String::from("bravo_value")), result);
    }

    #[test]
    fn should_get_paths_to_array_from_cbor() {
        let cbor_value = cbor!( {
            "alpha"  =>  {
                "bravo" => ["bravo_first_item", "bravo_second_item" ],
            }
        })
        .expect("valid cbor");
        let mut canonical: CborCanonicalMap = cbor_value.try_into().expect("valid canonical");
        let result = canonical
            .get_path_mut("alpha.bravo[0]")
            .expect("first item from bravo");
        assert_eq!(
            &mut CborValue::Text(String::from("bravo_first_item")),
            result
        );
    }

    #[test]
    fn should_return_non_when_path_not_exist() {
        let cbor_value = cbor!( {
            "alpha"  =>  {
                "bravo" => ["bravo_first_item", "bravo_second_item" ],
            }
        })
        .expect("valid cbor");
        let mut canonical: CborCanonicalMap = cbor_value.try_into().expect("valid canonical");
        let path = "alpha.bravo[-1]";

        assert!(canonical.get_path_mut(path).is_none())
    }

    #[test]
    fn should_replace_cbor_value() {
        let cbor_value = cbor!({
            "alpha"  =>  {
                "array_value" => vec![0_u8;32]

            }
        })
        .expect("cbor should be created");

        let mut canonical: CborCanonicalMap = cbor_value.try_into().expect("valid canonical");
        canonical.replace_path(
            "alpha.array_value",
            FieldType::ArrayInt,
            FieldType::StringBase58,
        );

        let replaced = canonical
            .get_path_mut("alpha.array_value")
            .expect("value should be returned");

        assert_eq!(
            &mut CborValue::Text(bs58::encode(vec![0_u8; 32]).into_string()),
            replaced
        );
    }
}
