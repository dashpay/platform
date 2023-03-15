use crate::value_map::{ValueMap, ValueMapHelper};
use crate::Identifier;
use crate::Value::Bool;
use crate::{Error, Value};
use std::collections::BTreeMap;

impl Value {
    pub fn has(&self, key: &str) -> Result<bool, Error> {
        self.get_optional_value(key).map(|v| v.is_some())
    }

    pub fn get<'a>(&'a self, key: &'a str) -> Result<Option<&'a Value>, Error> {
        self.get_optional_value(key)
    }

    pub fn get_mut<'a>(&'a mut self, key: &'a str) -> Result<Option<&'a mut Value>, Error> {
        self.get_optional_value_mut(key)
    }

    pub fn get_value<'a>(&'a self, key: &'a str) -> Result<&'a Value, Error> {
        let map = self.to_map()?;
        Self::get_from_map(map, key)
    }

    pub fn get_value_mut<'a>(&'a mut self, key: &'a str) -> Result<&'a mut Value, Error> {
        let map = self.to_map_mut()?;
        Self::get_mut_from_map(map, key)
    }

    pub fn get_optional_value<'a>(&'a self, key: &'a str) -> Result<Option<&'a Value>, Error> {
        let map = self.to_map()?;
        Ok(Self::get_optional_from_map(map, key))
    }

    pub fn get_optional_value_mut<'a>(
        &'a mut self,
        key: &'a str,
    ) -> Result<Option<&'a mut Value>, Error> {
        let map = self.to_map_mut()?;
        Ok(Self::get_optional_mut_from_map(map, key))
    }

    pub fn set_into_value<T>(&mut self, key: &str, value: T) -> Result<(), Error>
    where
        T: Into<Value>,
    {
        let map = self.as_map_mut_ref()?;
        Ok(Self::insert_in_map(map, key, value.into()))
    }

    pub fn set_value(&mut self, key: &str, value: Value) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        Ok(Self::insert_in_map(map, key, value))
    }

    pub fn insert(&mut self, key: String, value: Value) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        Ok(Self::insert_in_map_string_value(map, key, value))
    }

    pub fn remove(&mut self, key: &str) -> Result<Value, Error> {
        let map = self.as_map_mut_ref()?;
        map.remove_key(key)
    }

    pub fn remove_many(&mut self, keys: &Vec<&str>) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        keys.into_iter()
            .try_for_each(|key| map.remove_key(key).map(|_| ()))
    }

    pub fn remove_optional_value(&mut self, key: &str) -> Result<Option<Value>, Error> {
        let map = self.as_map_mut_ref()?;
        Ok(map.remove_optional_key(key))
    }

    pub fn remove_integer<T>(&mut self, key: &str) -> Result<T, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        let map = self.as_map_mut_ref()?;
        let value = map.remove_key(key)?;
        value.into_integer()
    }

    pub fn remove_optional_integer<T>(&mut self, key: &str) -> Result<Option<T>, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        let map = self.as_map_mut_ref()?;
        map.remove_optional_key(key)
            .map(|v| v.into_integer())
            .transpose()
    }

    pub fn remove_hash256_bytes(&mut self, key: &str) -> Result<[u8; 32], Error> {
        let map = self.as_map_mut_ref()?;
        let value = map.remove_key(key)?;
        value.into_hash256()
    }

    pub fn remove_optional_hash256_bytes(&mut self, key: &str) -> Result<Option<[u8; 32]>, Error> {
        let map = self.as_map_mut_ref()?;
        map.remove_optional_key(key)
            .map(|v| v.into_hash256())
            .transpose()
    }

    pub fn remove_bytes(&mut self, key: &str) -> Result<Vec<u8>, Error> {
        let map = self.as_map_mut_ref()?;
        let value = map.remove_key(key)?;
        value.into_bytes()
    }

    pub fn remove_optional_bytes(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let map = self.as_map_mut_ref()?;
        map.remove_optional_key(key)
            .map(|v| v.into_bytes())
            .transpose()
    }

    pub fn remove_array(&mut self, key: &str) -> Result<Vec<Value>, Error> {
        let map = self.as_map_mut_ref()?;
        let value = map.remove_key(key)?;
        value.into_array()
    }

    pub fn remove_optional_array<T>(&mut self, key: &str) -> Result<Option<Vec<Value>>, Error> {
        let map = self.as_map_mut_ref()?;
        map.remove_optional_key(key)
            .map(|v| v.into_array())
            .transpose()
    }

    pub fn get_optional_integer<T>(&self, key: &str) -> Result<Option<T>, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        let map = self.to_map()?;
        Self::inner_optional_integer_value(map, key)
    }

    pub fn get_integer<T>(&self, key: &str) -> Result<T, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        let map = self.to_map()?;
        Self::inner_integer_value(map, key)
    }

    pub fn get_optional_str<'a>(&'a self, key: &'a str) -> Result<Option<&'a str>, Error> {
        let map = self.to_map()?;
        Self::inner_optional_text_value(map, key)
    }

    pub fn get_str<'a>(&'a self, key: &'a str) -> Result<&'a str, Error> {
        let map = self.to_map()?;
        Self::inner_text_value(map, key)
    }

    pub fn get_optional_bool(&self, key: &str) -> Result<Option<bool>, Error> {
        let map = self.to_map()?;
        Self::inner_optional_bool_value(map, key)
    }

    pub fn get_bool<'a>(&'a self, key: &'a str) -> Result<bool, Error> {
        let map = self.to_map()?;
        Self::inner_bool_value(map, key)
    }

    pub fn get_optional_array(&self, key: &str) -> Result<Option<Vec<Value>>, Error> {
        let map = self.to_map()?;
        Self::inner_optional_array(map, key)
    }

    pub fn get_array<'a>(&'a self, key: &'a str) -> Result<Vec<Value>, Error> {
        let map = self.to_map()?;
        Self::inner_array_owned(map, key)
    }

    pub fn get_optional_string_ref_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        key: &'a str,
    ) -> Result<Option<I>, Error> {
        let map = self.to_map()?;
        Self::inner_optional_string_ref_map(map, key)
    }

    pub fn get_string_ref_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        key: &'a str,
    ) -> Result<I, Error> {
        let map = self.to_map()?;
        Self::inner_string_ref_map(map, key)
    }

    pub fn get_optional_string_mut_ref_map<'a, I: FromIterator<(String, &'a mut Value)>>(
        &'a mut self,
        key: &'a str,
    ) -> Result<Option<I>, Error> {
        let map = self.to_map_mut()?;
        Self::inner_optional_string_mut_ref_map(map, key)
    }

    pub fn get_string_mut_ref_map<'a, I: FromIterator<(String, &'a mut Value)>>(
        &'a mut self,
        key: &'a str,
    ) -> Result<I, Error> {
        let map = self.to_map_mut()?;
        Self::inner_string_mut_ref_map(map, key)
    }

    // pub fn get_array_into<'a, T: TryFrom<Value>>(&'a self, key: &'a str) -> Result<Vec<T>, Error> {
    //     let map = self.to_map()?;
    //     Self::inner_array(map, key).and_then(|vec | vec.into_iter().map(|value| value.try_into()).collect::<Result<Vec<T>, Error>>())
    // }
    //
    // pub fn get_optional_array_into<'a, T: TryFrom<Value>>(&'a self, key: &'a str) -> Result<Option<Vec<T>>, Error> {
    //     let map = self.to_map()?;
    //     Self::inner_optional_array(map, key)?.map(|vec | vec.into_iter().map(|value| value.try_into()).collect::<Result<Vec<T>, Error>>()).transpose()
    // }

    pub fn get_optional_array_slice<'a>(
        &'a self,
        key: &'a str,
    ) -> Result<Option<&'a [Value]>, Error> {
        let map = self.to_map()?;
        Self::inner_optional_array_slice(map, key)
    }

    pub fn get_array_ref<'a>(&'a self, key: &'a str) -> Result<&'a Vec<Value>, Error> {
        let map = self.to_map()?;
        Self::inner_array_ref(map, key)
    }

    pub fn get_array_mut_ref<'a>(&'a mut self, key: &'a str) -> Result<&'a mut Vec<Value>, Error> {
        let map = self.to_map_mut()?;
        Self::inner_array_mut_ref(map, key)
    }

    pub fn get_array_slice<'a>(&'a self, key: &'a str) -> Result<&[Value], Error> {
        let map = self.to_map()?;
        Self::inner_array_slice(map, key)
    }

    pub fn get_optional_bytes<'a>(&'a self, key: &'a str) -> Result<Option<Vec<u8>>, Error> {
        let map = self.to_map()?;
        Self::inner_optional_bytes_value(map, key)
    }

    pub fn get_bytes<'a>(&'a self, key: &'a str) -> Result<Vec<u8>, Error> {
        let map = self.to_map()?;
        Self::inner_bytes_value(map, key)
    }

    pub fn get_optional_bytes_into<T: From<Vec<u8>>>(&self, key: &str) -> Result<Option<T>, Error> {
        let map = self.to_map()?;
        Ok(Self::inner_optional_bytes_value(map, key)?.map(|bytes| bytes.into()))
    }

    pub fn get_optional_bytes_try_into<T: TryFrom<Vec<u8>, Error = Error>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Error> {
        let map = self.to_map()?;
        Self::inner_optional_bytes_value(map, key)?
            .map(|bytes| bytes.try_into())
            .transpose()
    }

    pub fn get_bytes_into<T: From<Vec<u8>>>(&self, key: &str) -> Result<T, Error> {
        let map = self.to_map()?;
        Ok(Self::inner_bytes_value(map, key)?.into())
    }

    pub fn get_bytes_try_into<T: TryFrom<Vec<u8>, Error = Error>>(
        &self,
        key: &str,
    ) -> Result<T, Error> {
        let map = self.to_map()?;
        Self::inner_bytes_value(map, key)?.try_into()
    }

    pub fn get_optional_hash256<'a>(&'a self, key: &'a str) -> Result<Option<[u8; 32]>, Error> {
        let map = self.to_map()?;
        Self::inner_optional_hash256_value(map, key)
    }

    pub fn get_identifier<'a>(&'a self, key: &'a str) -> Result<Identifier, Error> {
        let map = self.to_map()?;
        Ok(Identifier::new(Self::inner_hash256_value(map, key)?))
    }

    pub fn get_optional_identifier<'a>(
        &'a self,
        key: &'a str,
    ) -> Result<Option<Identifier>, Error> {
        let map = self.to_map()?;
        Ok(Self::inner_optional_hash256_value(map, key)?
            .map(|identifier| Identifier::new(identifier)))
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

    /// Retrieves the value of a key from a map if it's an array of strings.
    pub fn inner_optional_array(
        document_type: &[(Value, Value)],
        key: &str,
    ) -> Result<Option<Vec<Value>>, Error> {
        Self::get_optional_from_map(document_type, key)
            .map(|value| value.to_array_owned())
            .transpose()
    }

    /// Retrieves the value of a key from a map if it's an array of strings.
    pub fn inner_array_mut_ref<'a>(
        document_type: &'a mut [(Value, Value)],
        key: &'a str,
    ) -> Result<&'a mut Vec<Value>, Error> {
        Self::get_mut_from_map(document_type, key).map(|value| value.to_array_mut())?
    }

    /// Retrieves the value of a key from a map if it's an array of strings.
    pub fn inner_array_ref<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<&'a Vec<Value>, Error> {
        Self::get_from_map(document_type, key).map(|value| value.to_array_ref())?
    }

    /// Retrieves the value of a key from a map if it's an array of strings.
    pub fn inner_array_owned(
        document_type: &[(Value, Value)],
        key: &str,
    ) -> Result<Vec<Value>, Error> {
        Self::get_from_map(document_type, key).map(|value| value.to_array_owned())?
    }

    /// Retrieves the value of a key from a map if it's an array of strings.
    pub fn inner_optional_array_slice<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<&'a [Value]>, Error> {
        Self::get_optional_from_map(document_type, key)
            .map(|value| value.to_array_slice())
            .transpose()
    }

    /// Retrieves the value of a key from a map if it's an array of strings.
    pub fn inner_array_slice<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<&'a [Value], Error> {
        Self::get_from_map(document_type, key).map(|value| value.to_array_slice())?
    }

    /// Gets the inner map from a map and converts it to a string map
    pub fn inner_string_ref_map<'a, I: FromIterator<(String, &'a Value)>>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<I, Error> {
        Self::get_from_map(document_type, key).map(|value| value.to_ref_string_map())?
    }

    /// Gets the inner map from a map and converts it to a string map
    pub fn inner_optional_string_ref_map<'a, I: FromIterator<(String, &'a Value)>>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<I>, Error> {
        let Some(key_value) = Self::get_optional_from_map(document_type, key) else {
            return Ok(None);
        };
        if let Value::Map(map_value) = key_value {
            return Ok(Some(Value::map_ref_into_string_map(map_value)?));
        }
        Ok(None)
    }

    /// Gets the inner map from a map and converts it to a string map
    pub fn inner_string_mut_ref_map<'a, I: FromIterator<(String, &'a mut Value)>>(
        document_type: &'a mut [(Value, Value)],
        key: &'a str,
    ) -> Result<I, Error> {
        Self::get_mut_from_map(document_type, key).map(|value| value.to_ref_string_map_mut())?
    }

    /// Gets the inner map from a map and converts it to a string map
    pub fn inner_optional_string_mut_ref_map<'a, I: FromIterator<(String, &'a mut Value)>>(
        document_type: &'a mut [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<I>, Error> {
        let Some(key_value) = Self::get_optional_mut_from_map(document_type, key) else {
            return Ok(None);
        };
        Ok(Some(key_value.to_ref_string_map_mut()?))
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
            return Ok(Some(Value::map_ref_into_btree_string_map(map_value)?));
        }
        Ok(None)
    }

    /// Gets the inner bool value from a map
    pub fn inner_optional_bool_value(
        document_type: &[(Value, Value)],
        key: &str,
    ) -> Result<Option<bool>, Error> {
        Self::get_optional_from_map(document_type, key)
            .map(|value| value.to_bool())
            .transpose()
    }

    /// Gets the inner bool value from a map
    pub fn inner_bool_value(document_type: &[(Value, Value)], key: &str) -> Result<bool, Error> {
        Self::get_from_map(document_type, key).map(|value| value.to_bool())?
    }

    /// Gets the inner integer value from a map if it exists
    pub fn inner_optional_integer_value<T>(
        document_type: &[(Value, Value)],
        key: &str,
    ) -> Result<Option<T>, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        Self::get_optional_from_map(document_type, key)
            .map(|key_value| key_value.to_integer())
            .transpose()
    }

    /// Gets the inner integer value from a map
    pub fn inner_integer_value<T>(document_type: &[(Value, Value)], key: &str) -> Result<T, Error>
    where
        T: TryFrom<i128>
            + TryFrom<u128>
            + TryFrom<u64>
            + TryFrom<i64>
            + TryFrom<u32>
            + TryFrom<i32>
            + TryFrom<u16>
            + TryFrom<i16>
            + TryFrom<u8>
            + TryFrom<i8>,
    {
        let key_value = Self::get_from_map(document_type, key)?;
        key_value.to_integer()
    }

    /// Retrieves the value of a key from a map if it's a string.
    pub fn inner_optional_text_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<&'a str>, Error> {
        Self::get_optional_from_map(document_type, key)
            .map(|v| v.to_str())
            .transpose()
    }

    /// Retrieves the value of a key from a map if it's a string.
    pub fn inner_text_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<&'a str, Error> {
        Self::get_from_map(document_type, key).map(|v| v.to_str())?
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
    pub fn inner_bytes_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Vec<u8>, Error> {
        Self::get_from_map(document_type, key).map(|v| v.to_bytes())?
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

    pub fn get_mut_from_map<'a>(
        map: &'a mut [(Value, Value)],
        search_key: &'a str,
    ) -> Result<&'a mut Value, Error> {
        Self::get_optional_mut_from_map(map, search_key).ok_or(Error::StructureError(format!(
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

    /// Gets a value from a map
    pub fn get_optional_mut_from_map<'a>(
        map: &'a mut [(Value, Value)],
        search_key: &'a str,
    ) -> Option<&'a mut Value> {
        for (key, value) in map.iter_mut() {
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

    /// Inserts into a map
    /// If the element already existed it will replace it
    pub fn insert_in_map_string_value(
        map: &mut ValueMap,
        inserting_key: String,
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
            map.push((Value::Text(inserting_key), inserting_value))
        }
    }
}
