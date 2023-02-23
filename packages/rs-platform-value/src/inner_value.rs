use crate::{Error, Value};
use std::collections::BTreeMap;

impl Value {
    /// Retrieves the value of a key from a map if it's an array of strings.
    pub fn inner_array_of_strings<'a, I: FromIterator<String>>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Option<I> {
        let key_value = Self::get_from_map(document_type, key)?;
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
    pub fn inner_btree_map<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<BTreeMap<String, &'a Value>>, Error> {
        let Some(key_value) = Self::get_from_map(document_type, key) else {
            return Ok(None);
        };
        if let Value::Map(map_value) = key_value {
            return Ok(Some(Value::map_ref_into_btree_map(map_value)?));
        }
        Ok(None)
    }

    /// Gets the inner bool value from a map
    pub fn inner_bool_value(document_type: &[(Value, Value)], key: &str) -> Option<bool> {
        let key_value = Self::get_from_map(document_type, key)?;
        if let Value::Bool(bool_value) = key_value {
            return Some(*bool_value);
        }
        None
    }

    /// Retrieves the value of a key from a map if it's a string.
    pub fn inner_text_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<&'a str>, Error> {
        Self::get_from_map(document_type, key)
            .map(|v| v.as_str())
            .transpose()
    }

    /// Retrieves the value of a key from a map if it's a byte array.
    pub fn inner_bytes_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<Vec<u8>>, Error> {
        Self::get_from_map(document_type, key)
            .map(|v| v.to_bytes())
            .transpose()
    }

    /// Retrieves the value of a key from a map if it's a byte array.
    pub fn inner_bytes_slice_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<&'a [u8]>, Error> {
        Self::get_from_map(document_type, key)
            .map(|v| v.as_bytes_slice())
            .transpose()
    }

    /// Gets the inner array value from a borrowed ValueMap
    pub fn inner_array_slice_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<&'a [Value]>, Error> {
        Self::get_from_map(document_type, key)
            .map(|v| v.as_slice())
            .transpose()
    }

    /// Gets a value from a map
    pub fn get_from_map<'a>(map: &'a [(Value, Value)], search_key: &'a str) -> Option<&'a Value> {
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
}
