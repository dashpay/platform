use crate::value_map::{ValueMap, ValueMapHelper};
use crate::Value::Bool;
use crate::{Error, Value};
use std::collections::BTreeMap;

impl Value {
    pub fn get_value<'a>(&'a self, key: &'a str) -> Result<&'a Value, Error> {
        let map = self.to_map()?;
        Self::get_from_map(map, key)
    }

    pub fn set_value(&mut self, key: &str, value: Value) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        Ok(Self::insert_in_map(map, key, value))
    }

    pub fn remove_value(&mut self, key: &str) -> Result<Option<Value>, Error> {
        let map = self.as_map_mut_ref()?;
        Ok(map.remove_key(key))
    }

    pub fn get_string<'a>(&'a self, key: &'a str) -> Result<&'a str, Error> {
        let map = self.to_map()?;
        Self::inner_text_value(map, key)
    }

    pub fn get_hash256<'a>(&'a self, key: &'a str) -> Result<[u8; 32], Error> {
        let map = self.to_map()?;
        Self::inner_hash256_value(map, key)
    }

    pub fn get_hash256_as_bs58_string<'a>(&'a self, key: &'a str) -> Result<String, Error> {
        let map = self.to_map()?;
        let value = Self::inner_hash256_value(map, key)?;
        Ok(bs58::encode(value).into_string())
    }

    pub fn get_optional_value<'a>(&'a self, key: &'a str) -> Result<Option<&Value>, Error> {
        let map = self.to_map()?;
        Ok(Self::get_optional_from_map(map, key))
    }

    /// Retrieves the value of a key from a map if it's an array of strings.
    pub fn inner_optional_array_of_strings<'a, I: FromIterator<String>>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Option<I> {
        let key_value = Self::get_optional_from_map(document_type, key)?;
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

    /// Gets the inner btree map from a map
    pub fn inner_optional_btree_map<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<BTreeMap<String, &'a Value>>, Error> {
        let Some(key_value) = Self::get_optional_from_map(document_type, key) else {
            return Ok(None);
        };
        if let Value::Map(map_value) = key_value {
            return Ok(Some(Value::map_ref_into_btree_map(map_value)?));
        }
        Ok(None)
    }

    /// Gets the inner bool value from a map
    pub fn inner_optional_bool_value(document_type: &[(Value, Value)], key: &str) -> Option<bool> {
        let key_value = Self::get_optional_from_map(document_type, key)?;
        if let Value::Bool(bool_value) = key_value {
            return Some(*bool_value);
        }
        None
    }

    /// Retrieves the value of a key from a map if it's a string.
    pub fn inner_optional_text_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<&'a str>, Error> {
        Self::get_optional_from_map(document_type, key)
            .map(|v| v.as_str())
            .transpose()
    }

    /// Retrieves the value of a key from a map if it's a string.
    pub fn inner_text_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<&'a str, Error> {
        Self::get_from_map(document_type, key).map(|v| v.as_str())?
    }

    /// Retrieves the value of a key from a map if it's a hash256.
    pub fn inner_optional_hash256_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<[u8; 32]>, Error> {
        Self::get_optional_from_map(document_type, key)
            .map(|v| v.to_hash256())
            .transpose()
    }

    /// Retrieves the value of a key from a map if it's a string.
    pub fn inner_hash256_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<[u8; 32], Error> {
        Self::get_from_map(document_type, key).map(|v| v.to_hash256())?
    }

    /// Retrieves the value of a key from a map if it's a byte array.
    pub fn inner_optional_bytes_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<Vec<u8>>, Error> {
        Self::get_optional_from_map(document_type, key)
            .map(|v| v.to_bytes())
            .transpose()
    }

    /// Retrieves the value of a key from a map if it's a byte array.
    pub fn inner_optional_bytes_slice_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<&'a [u8]>, Error> {
        Self::get_optional_from_map(document_type, key)
            .map(|v| v.as_bytes_slice())
            .transpose()
    }

    /// Gets the inner array value from a borrowed ValueMap
    pub fn inner_optional_array_slice_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<&'a [Value]>, Error> {
        Self::get_optional_from_map(document_type, key)
            .map(|v| v.as_slice())
            .transpose()
    }

    pub fn get_from_map<'a>(
        map: &'a [(Value, Value)],
        search_key: &'a str,
    ) -> Result<&'a Value, Error> {
        Self::get_optional_from_map(map, search_key).ok_or(Error::StructureError(format!(
            "{} not found in map",
            search_key
        )))
    }

    /// Gets a value from a map
    pub fn get_optional_from_map<'a>(
        map: &'a [(Value, Value)],
        search_key: &'a str,
    ) -> Option<&'a Value> {
        for (key, value) in map.iter() {
            if !key.is_text() {
                continue;
            }

            if key.as_text().expect("confirmed as text") == search_key {
                return Some(value);
            }
        }
        None
    }

    /// Inserts into a map
    /// If the element already existed it will replace it
    pub fn insert_in_map<'a>(
        map: &'a mut ValueMap,
        inserting_key: &'a str,
        inserting_value: Value,
    ) {
        let mut found_value = None;
        for (key, value) in map.iter_mut() {
            if !key.is_text() {
                continue;
            }

            if key.as_text().expect("confirmed as text") == inserting_key {
                found_value = Some(value);
                break;
            }
        }
        if let Some(value) = found_value {
            *value = inserting_value;
        } else {
            map.push((Value::Text(inserting_key.to_string()), inserting_value))
        }
    }
}
