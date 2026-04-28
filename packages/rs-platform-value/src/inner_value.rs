use crate::value_map::{ValueMap, ValueMapHelper};
use crate::{BinaryData, Bytes32, Identifier};
use crate::{Error, Value};
use indexmap::IndexMap;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

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
        Self::insert_in_map(map, key, value.into());
        Ok(())
    }

    pub fn set_into_binary_data(&mut self, key: &str, value: Vec<u8>) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        Self::insert_in_map(map, key, Value::Bytes(value));
        Ok(())
    }

    pub fn set_value(&mut self, key: &str, value: Value) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        Self::insert_in_map(map, key, value);
        Ok(())
    }

    pub fn insert(&mut self, key: String, value: Value) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        Self::insert_in_map_string_value(map, key, value);
        Ok(())
    }

    pub fn insert_at_end(&mut self, key: String, value: Value) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        Self::push_to_map_string_value(map, key, value);
        Ok(())
    }

    pub fn remove(&mut self, key: &str) -> Result<Value, Error> {
        let map = self.as_map_mut_ref()?;
        map.remove_key(key)
    }

    pub fn remove_many(&mut self, keys: &[&str]) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        keys.iter()
            .try_for_each(|key| map.remove_key(key).map(|_| ()))
    }

    pub fn remove_optional_value(&mut self, key: &str) -> Result<Option<Value>, Error> {
        let map = self.as_map_mut_ref()?;
        Ok(map.remove_optional_key(key))
    }

    pub fn remove_optional_value_if_null(&mut self, key: &str) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        map.remove_optional_key_if_null(key);
        Ok(())
    }

    pub fn remove_optional_value_if_empty_array(&mut self, key: &str) -> Result<(), Error> {
        let map = self.as_map_mut_ref()?;
        map.remove_optional_key_if_empty_array(key);
        Ok(())
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
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_integer())
                }
            })
            .transpose()
    }

    pub fn remove_identifier(&mut self, key: &str) -> Result<Identifier, Error> {
        let map = self.as_map_mut_ref()?;
        let value = map.remove_key(key)?;
        value.into_identifier()
    }

    pub fn remove_optional_identifier(&mut self, key: &str) -> Result<Option<Identifier>, Error> {
        let map = self.as_map_mut_ref()?;
        map.remove_optional_key(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_identifier())
                }
            })
            .transpose()
    }

    pub fn remove_bytes_32(&mut self, key: &str) -> Result<Bytes32, Error> {
        let map = self.as_map_mut_ref()?;
        let value = map.remove_key(key)?;
        value.into_bytes_32()
    }

    pub fn remove_optional_bytes_32(&mut self, key: &str) -> Result<Option<Bytes32>, Error> {
        let map = self.as_map_mut_ref()?;
        map.remove_optional_key(key)
            .map(|v| v.into_bytes_32())
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

    pub fn remove_binary_data(&mut self, key: &str) -> Result<BinaryData, Error> {
        let map = self.as_map_mut_ref()?;
        let value = map.remove_key(key)?;
        value.into_binary_data()
    }

    pub fn remove_optional_binary_data(&mut self, key: &str) -> Result<Option<BinaryData>, Error> {
        let map = self.as_map_mut_ref()?;
        map.remove_optional_key(key)
            .map(|v| v.into_binary_data())
            .transpose()
    }

    pub fn remove_array(&mut self, key: &str) -> Result<Vec<Value>, Error> {
        let map = self.as_map_mut_ref()?;
        let value = map.remove_key(key)?;
        value.into_array()
    }

    pub fn remove_optional_array(&mut self, key: &str) -> Result<Option<Vec<Value>>, Error> {
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

    pub fn get_optional_array_mut_ref<'a>(
        &'a mut self,
        key: &'a str,
    ) -> Result<Option<&'a mut Vec<Value>>, Error> {
        let map = self.to_map_mut()?;
        Self::inner_optional_array_mut_ref(map, key)
    }

    pub fn get_array_mut_ref<'a>(&'a mut self, key: &'a str) -> Result<&'a mut Vec<Value>, Error> {
        let map = self.to_map_mut()?;
        Self::inner_array_mut_ref(map, key)
    }

    pub fn get_array_slice<'a>(&'a self, key: &'a str) -> Result<&'a [Value], Error> {
        let map = self.to_map()?;
        Self::inner_array_slice(map, key)
    }

    pub fn get_optional_binary_data<'a>(
        &'a self,
        key: &'a str,
    ) -> Result<Option<BinaryData>, Error> {
        let map = self.to_map()?;
        Self::inner_optional_binary_data_value(map, key)
    }

    pub fn get_binary_data<'a>(&'a self, key: &'a str) -> Result<BinaryData, Error> {
        let map = self.to_map()?;
        Self::inner_binary_data_value(map, key)
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
        Ok(Self::inner_optional_hash256_value(map, key)?.map(Identifier::new))
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
    /// This is useful for constructing the required fields in a contract.
    /// For the DPNS Contract we would get as a result
    /// {
    ///     "label",
    ///     "normalizedLabel",
    ///     "normalizedParentDomainName",
    ///     "preorderSalt",
    ///     "records",
    ///     "subdomainRules",
    ///     "subdomainRules.allowSubdomains",
    /// }
    pub fn inner_recursive_optional_array_of_strings<'a>(
        document_type: &'a [(Value, Value)],
        prefix: String,
        recursive_key: &'a str,
        key: &'a str,
    ) -> BTreeSet<String> {
        let mut result = if let Some(Value::Array(key_value)) =
            Self::get_optional_from_map(document_type, key)
        {
            key_value
                .iter()
                .filter_map(|v| {
                    if let Value::Text(text) = v {
                        Some(format!("{prefix}{text}"))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            BTreeSet::new()
        };
        if let Some(Value::Map(lower_level)) =
            Self::get_optional_from_map(document_type, recursive_key)
        {
            for (inner_key, value) in lower_level {
                let level_prefix = if let Value::Text(text) = inner_key {
                    text.as_str()
                } else {
                    continue;
                };
                let Value::Map(level_map) = value else {
                    continue;
                };

                let prefix = format!("{prefix}{level_prefix}.");
                result.extend(Self::inner_recursive_optional_array_of_strings(
                    level_map,
                    prefix,
                    recursive_key,
                    key,
                ))
            }
        }
        result
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
    pub fn inner_optional_array_mut_ref<'a>(
        document_type: &'a mut [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<&'a mut Vec<Value>>, Error> {
        Self::get_optional_mut_from_map(document_type, key)
            .map(|value| value.to_array_mut())
            .transpose()
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

    /// Gets the inner index map sorted by a specified property
    pub fn inner_optional_index_map<'a, T>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
        sort_property: &'a str,
    ) -> Result<Option<IndexMap<String, &'a Value>>, Error>
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
            + TryFrom<i8>
            + Ord,
    {
        let Some(key_value) = Self::get_optional_from_map(document_type, key) else {
            return Ok(None);
        };
        if let Value::Map(map_value) = key_value {
            return Ok(Some(Value::map_ref_into_indexed_string_map::<T>(
                map_value,
                sort_property,
            )?));
        }
        Ok(None)
    }

    /// Gets the inner bool value from a map
    pub fn inner_optional_bool_value(
        document_type: &[(Value, Value)],
        key: &str,
    ) -> Result<Option<bool>, Error> {
        Self::get_optional_from_map(document_type, key)
            .and_then(|value| {
                if value.is_null() {
                    None
                } else {
                    Some(value.to_bool())
                }
            })
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
            .and_then(|key_value| {
                if key_value.is_null() {
                    None
                } else {
                    Some(key_value.to_integer())
                }
            })
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
    pub fn inner_optional_binary_data_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<Option<BinaryData>, Error> {
        Self::get_optional_from_map(document_type, key)
            .map(|v| v.to_binary_data())
            .transpose()
    }

    /// Retrieves the value of a key from a map if it's a byte array.
    pub fn inner_binary_data_value<'a>(
        document_type: &'a [(Value, Value)],
        key: &'a str,
    ) -> Result<BinaryData, Error> {
        Self::get_from_map(document_type, key).map(|v| v.to_binary_data())?
    }

    /// Retrieves the val
    ///
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
        Self::get_optional_from_map(map, search_key)
            .ok_or_else(|| Error::StructureError(format!("{} not found in map", search_key)))
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
                return if value.is_null() { None } else { Some(value) };
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
                return if value.is_null() { None } else { Some(value) };
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
        let mut pos = 0;
        for (key, value) in map.iter_mut() {
            if let Value::Text(text) = key {
                match inserting_key.cmp(text) {
                    Ordering::Less => {}
                    Ordering::Equal => {
                        found_value = Some(value);
                        break;
                    }
                    Ordering::Greater => {
                        pos += 1;
                    }
                }
            }
        }
        if let Some(value) = found_value {
            *value = inserting_value;
        } else {
            map.insert(pos, (Value::Text(inserting_key), inserting_value))
        }
    }

    /// Inserts into a map
    /// If the element already existed it will replace it
    pub fn push_to_map_string_value(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a Value::Map with string keys for testing
    fn make_map(entries: Vec<(&str, Value)>) -> Value {
        Value::Map(
            entries
                .into_iter()
                .map(|(k, v)| (Value::Text(k.to_string()), v))
                .collect(),
        )
    }

    // ---------------------------------------------------------------
    // has / get / get_mut
    // ---------------------------------------------------------------

    #[test]
    fn has_returns_true_for_existing_key() {
        let val = make_map(vec![("name", Value::Text("alice".into()))]);
        assert!(val.has("name").unwrap());
    }

    #[test]
    fn has_returns_false_for_missing_key() {
        let val = make_map(vec![("name", Value::Text("alice".into()))]);
        assert!(!val.has("age").unwrap());
    }

    #[test]
    fn has_errors_on_non_map() {
        let val = Value::U64(42);
        assert!(val.has("key").is_err());
    }

    #[test]
    fn get_returns_some_for_existing_key() {
        let val = make_map(vec![("x", Value::U32(7))]);
        let result = val.get("x").unwrap();
        assert_eq!(result, Some(&Value::U32(7)));
    }

    #[test]
    fn get_returns_none_for_missing_key() {
        let val = make_map(vec![("x", Value::U32(7))]);
        assert_eq!(val.get("y").unwrap(), None);
    }

    #[test]
    fn get_mut_returns_mutable_ref() {
        let mut val = make_map(vec![("x", Value::U32(7))]);
        let inner = val.get_mut("x").unwrap().unwrap();
        *inner = Value::U32(99);
        assert_eq!(val.get("x").unwrap(), Some(&Value::U32(99)));
    }

    #[test]
    fn get_value_returns_ref() {
        let val = make_map(vec![("k", Value::Bool(true))]);
        assert_eq!(val.get_value("k").unwrap(), &Value::Bool(true));
    }

    #[test]
    fn get_value_errors_on_missing_key() {
        let val = make_map(vec![("k", Value::Bool(true))]);
        assert!(val.get_value("missing").is_err());
    }

    #[test]
    fn get_value_mut_modifies_value() {
        let mut val = make_map(vec![("k", Value::Bool(true))]);
        let inner = val.get_value_mut("k").unwrap();
        *inner = Value::Bool(false);
        assert_eq!(val.get_value("k").unwrap(), &Value::Bool(false));
    }

    // ---------------------------------------------------------------
    // set_into_value / set_value / insert / insert_at_end
    // ---------------------------------------------------------------

    #[test]
    fn set_into_value_adds_new_key() {
        let mut val = make_map(vec![]);
        val.set_into_value("age", 42u64).unwrap();
        let result: u64 = val.get_integer("age").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn set_into_value_replaces_existing() {
        let mut val = make_map(vec![("age", Value::U64(10))]);
        val.set_into_value("age", 42u64).unwrap();
        let result: u64 = val.get_integer("age").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn set_into_binary_data_works() {
        let mut val = make_map(vec![]);
        val.set_into_binary_data("data", vec![1, 2, 3]).unwrap();
        assert_eq!(val.get_value("data").unwrap(), &Value::Bytes(vec![1, 2, 3]));
    }

    #[test]
    fn set_value_works() {
        let mut val = make_map(vec![]);
        val.set_value("flag", Value::Bool(true)).unwrap();
        assert_eq!(val.get_value("flag").unwrap(), &Value::Bool(true));
    }

    #[test]
    fn insert_sorted_position() {
        let mut val = make_map(vec![("b", Value::U8(2))]);
        val.insert("a".to_string(), Value::U8(1)).unwrap();
        // After insert, "a" should exist
        assert_eq!(val.get_value("a").unwrap(), &Value::U8(1));
    }

    #[test]
    fn insert_at_end_appends() {
        let mut val = make_map(vec![("a", Value::U8(1))]);
        val.insert_at_end("z".to_string(), Value::U8(26)).unwrap();
        assert_eq!(val.get_value("z").unwrap(), &Value::U8(26));
    }

    // ---------------------------------------------------------------
    // remove / remove_many / remove_optional_value
    // ---------------------------------------------------------------

    #[test]
    fn remove_existing_key() {
        let mut val = make_map(vec![("k", Value::U64(1))]);
        let removed = val.remove("k").unwrap();
        assert_eq!(removed, Value::U64(1));
        assert!(!val.has("k").unwrap());
    }

    #[test]
    fn remove_missing_key_errors() {
        let mut val = make_map(vec![("k", Value::U64(1))]);
        assert!(val.remove("missing").is_err());
    }

    #[test]
    fn remove_many_removes_all() {
        let mut val = make_map(vec![
            ("a", Value::U8(1)),
            ("b", Value::U8(2)),
            ("c", Value::U8(3)),
        ]);
        val.remove_many(&["a", "c"]).unwrap();
        assert!(!val.has("a").unwrap());
        assert!(val.has("b").unwrap());
        assert!(!val.has("c").unwrap());
    }

    #[test]
    fn remove_optional_value_returns_some() {
        let mut val = make_map(vec![("k", Value::U64(1))]);
        let result = val.remove_optional_value("k").unwrap();
        assert_eq!(result, Some(Value::U64(1)));
    }

    #[test]
    fn remove_optional_value_returns_none_for_missing() {
        let mut val = make_map(vec![("k", Value::U64(1))]);
        let result = val.remove_optional_value("missing").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn remove_optional_value_if_null_removes_null() {
        let mut val = make_map(vec![("k", Value::Null)]);
        // Note: Null entries return None from get_optional_from_map, so we set directly
        val.remove_optional_value_if_null("k").unwrap();
    }

    #[test]
    fn remove_optional_value_if_empty_array_removes() {
        let mut val = make_map(vec![("k", Value::Array(vec![]))]);
        val.remove_optional_value_if_empty_array("k").unwrap();
    }

    // ---------------------------------------------------------------
    // remove_integer / remove_optional_integer
    // ---------------------------------------------------------------

    #[test]
    fn remove_integer_works() {
        let mut val = make_map(vec![("n", Value::U64(42))]);
        let result: u64 = val.remove_integer("n").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn remove_optional_integer_some() {
        let mut val = make_map(vec![("n", Value::U32(99))]);
        let result: Option<u32> = val.remove_optional_integer("n").unwrap();
        assert_eq!(result, Some(99));
    }

    #[test]
    fn remove_optional_integer_none_for_missing() {
        let mut val = make_map(vec![("n", Value::U32(99))]);
        let result: Option<u32> = val.remove_optional_integer("missing").unwrap();
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // remove_identifier / remove_optional_identifier
    // ---------------------------------------------------------------

    #[test]
    fn remove_identifier_works() {
        let id_bytes = [7u8; 32];
        let mut val = make_map(vec![("id", Value::Identifier(id_bytes))]);
        let result = val.remove_identifier("id").unwrap();
        assert_eq!(result, Identifier::new(id_bytes));
    }

    #[test]
    fn remove_optional_identifier_some() {
        let id_bytes = [8u8; 32];
        let mut val = make_map(vec![("id", Value::Identifier(id_bytes))]);
        let result = val.remove_optional_identifier("id").unwrap();
        assert_eq!(result, Some(Identifier::new(id_bytes)));
    }

    #[test]
    fn remove_optional_identifier_none_for_missing() {
        let mut val = make_map(vec![("other", Value::U8(0))]);
        let result = val.remove_optional_identifier("id").unwrap();
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // remove_bytes_32 / remove_optional_bytes_32
    // ---------------------------------------------------------------

    #[test]
    fn remove_bytes_32_works() {
        let data = [3u8; 32];
        let mut val = make_map(vec![("b", Value::Bytes32(data))]);
        let result = val.remove_bytes_32("b").unwrap();
        assert_eq!(result, Bytes32::new(data));
    }

    #[test]
    fn remove_optional_bytes_32_returns_some() {
        let data = [4u8; 32];
        let mut val = make_map(vec![("b", Value::Bytes32(data))]);
        let result = val.remove_optional_bytes_32("b").unwrap();
        assert_eq!(result, Some(Bytes32::new(data)));
    }

    #[test]
    fn remove_optional_bytes_32_returns_none_for_missing() {
        let mut val = make_map(vec![]);
        let result = val.remove_optional_bytes_32("b").unwrap();
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // remove_hash256_bytes / remove_optional_hash256_bytes
    // ---------------------------------------------------------------

    #[test]
    fn remove_hash256_bytes_works() {
        let data = [9u8; 32];
        let mut val = make_map(vec![("h", Value::Bytes32(data))]);
        let result = val.remove_hash256_bytes("h").unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn remove_optional_hash256_bytes_some() {
        let data = [10u8; 32];
        let mut val = make_map(vec![("h", Value::Bytes32(data))]);
        let result = val.remove_optional_hash256_bytes("h").unwrap();
        assert_eq!(result, Some(data));
    }

    #[test]
    fn remove_optional_hash256_bytes_none() {
        let mut val = make_map(vec![]);
        let result = val.remove_optional_hash256_bytes("h").unwrap();
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // remove_bytes / remove_optional_bytes
    // ---------------------------------------------------------------

    #[test]
    fn remove_bytes_works() {
        let mut val = make_map(vec![("b", Value::Bytes(vec![1, 2, 3]))]);
        let result = val.remove_bytes("b").unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn remove_optional_bytes_some() {
        let mut val = make_map(vec![("b", Value::Bytes(vec![4, 5]))]);
        let result = val.remove_optional_bytes("b").unwrap();
        assert_eq!(result, Some(vec![4, 5]));
    }

    #[test]
    fn remove_optional_bytes_none_for_missing() {
        let mut val = make_map(vec![]);
        let result = val.remove_optional_bytes("b").unwrap();
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // remove_binary_data / remove_optional_binary_data
    // ---------------------------------------------------------------

    #[test]
    fn remove_binary_data_works() {
        let mut val = make_map(vec![("d", Value::Bytes(vec![10, 20]))]);
        let result = val.remove_binary_data("d").unwrap();
        assert_eq!(result, BinaryData::new(vec![10, 20]));
    }

    #[test]
    fn remove_optional_binary_data_some() {
        let mut val = make_map(vec![("d", Value::Bytes(vec![30]))]);
        let result = val.remove_optional_binary_data("d").unwrap();
        assert_eq!(result, Some(BinaryData::new(vec![30])));
    }

    #[test]
    fn remove_optional_binary_data_none() {
        let mut val = make_map(vec![]);
        let result = val.remove_optional_binary_data("d").unwrap();
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // remove_array / remove_optional_array
    // ---------------------------------------------------------------

    #[test]
    fn remove_array_works() {
        let mut val = make_map(vec![(
            "arr",
            Value::Array(vec![Value::U8(1), Value::U8(2)]),
        )]);
        let result = val.remove_array("arr").unwrap();
        assert_eq!(result, vec![Value::U8(1), Value::U8(2)]);
    }

    #[test]
    fn remove_optional_array_some() {
        let mut val = make_map(vec![("arr", Value::Array(vec![Value::U8(3)]))]);
        let result = val.remove_optional_array("arr").unwrap();
        assert_eq!(result, Some(vec![Value::U8(3)]));
    }

    #[test]
    fn remove_optional_array_none() {
        let mut val = make_map(vec![]);
        let result = val.remove_optional_array("arr").unwrap();
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // get_integer / get_optional_integer
    // ---------------------------------------------------------------

    #[test]
    fn get_integer_works() {
        let val = make_map(vec![("n", Value::I64(-5))]);
        let result: i64 = val.get_integer("n").unwrap();
        assert_eq!(result, -5);
    }

    #[test]
    fn get_integer_wrong_type() {
        let val = make_map(vec![("n", Value::Text("hello".into()))]);
        let result: Result<i64, Error> = val.get_integer("n");
        assert!(result.is_err());
    }

    #[test]
    fn get_optional_integer_some() {
        let val = make_map(vec![("n", Value::U16(100))]);
        let result: Option<u16> = val.get_optional_integer("n").unwrap();
        assert_eq!(result, Some(100));
    }

    #[test]
    fn get_optional_integer_none_for_missing() {
        let val = make_map(vec![]);
        let result: Option<u16> = val.get_optional_integer("n").unwrap();
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // get_str / get_optional_str
    // ---------------------------------------------------------------

    #[test]
    fn get_str_works() {
        let val = make_map(vec![("name", Value::Text("bob".into()))]);
        assert_eq!(val.get_str("name").unwrap(), "bob");
    }

    #[test]
    fn get_str_wrong_type() {
        let val = make_map(vec![("name", Value::U64(5))]);
        assert!(val.get_str("name").is_err());
    }

    #[test]
    fn get_optional_str_some() {
        let val = make_map(vec![("name", Value::Text("alice".into()))]);
        assert_eq!(val.get_optional_str("name").unwrap(), Some("alice"));
    }

    #[test]
    fn get_optional_str_none_for_missing() {
        let val = make_map(vec![]);
        assert_eq!(val.get_optional_str("name").unwrap(), None);
    }

    // ---------------------------------------------------------------
    // get_bool / get_optional_bool
    // ---------------------------------------------------------------

    #[test]
    fn get_bool_works() {
        let val = make_map(vec![("flag", Value::Bool(true))]);
        assert!(val.get_bool("flag").unwrap());
    }

    #[test]
    fn get_bool_wrong_type() {
        let val = make_map(vec![("flag", Value::U8(1))]);
        assert!(val.get_bool("flag").is_err());
    }

    #[test]
    fn get_optional_bool_some() {
        let val = make_map(vec![("flag", Value::Bool(false))]);
        assert_eq!(val.get_optional_bool("flag").unwrap(), Some(false));
    }

    #[test]
    fn get_optional_bool_none_for_missing() {
        let val = make_map(vec![]);
        assert_eq!(val.get_optional_bool("flag").unwrap(), None);
    }

    // ---------------------------------------------------------------
    // get_array / get_optional_array / get_array_ref / get_array_slice
    // ---------------------------------------------------------------

    #[test]
    fn get_array_works() {
        let val = make_map(vec![("a", Value::Array(vec![Value::U8(1)]))]);
        let result = val.get_array("a").unwrap();
        assert_eq!(result, vec![Value::U8(1)]);
    }

    #[test]
    fn get_optional_array_some() {
        let val = make_map(vec![("a", Value::Array(vec![Value::U8(2)]))]);
        let result = val.get_optional_array("a").unwrap();
        assert_eq!(result, Some(vec![Value::U8(2)]));
    }

    #[test]
    fn get_optional_array_none() {
        let val = make_map(vec![]);
        let result = val.get_optional_array("a").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_array_ref_works() {
        let val = make_map(vec![("a", Value::Array(vec![Value::U8(3)]))]);
        let result = val.get_array_ref("a").unwrap();
        assert_eq!(result, &vec![Value::U8(3)]);
    }

    #[test]
    fn get_array_slice_works() {
        let val = make_map(vec![("a", Value::Array(vec![Value::U8(4)]))]);
        let result = val.get_array_slice("a").unwrap();
        assert_eq!(result, &[Value::U8(4)]);
    }

    #[test]
    fn get_optional_array_slice_some() {
        let val = make_map(vec![("a", Value::Array(vec![Value::U8(5)]))]);
        let result = val.get_optional_array_slice("a").unwrap();
        assert_eq!(result, Some([Value::U8(5)].as_slice()));
    }

    #[test]
    fn get_optional_array_slice_none() {
        let val = make_map(vec![]);
        let result = val.get_optional_array_slice("a").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_array_mut_ref_works() {
        let mut val = make_map(vec![("a", Value::Array(vec![Value::U8(6)]))]);
        let arr = val.get_array_mut_ref("a").unwrap();
        arr.push(Value::U8(7));
        assert_eq!(
            val.get_array("a").unwrap(),
            vec![Value::U8(6), Value::U8(7)]
        );
    }

    #[test]
    fn get_optional_array_mut_ref_some() {
        let mut val = make_map(vec![("a", Value::Array(vec![Value::U8(8)]))]);
        let result = val.get_optional_array_mut_ref("a").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn get_optional_array_mut_ref_none() {
        let mut val = make_map(vec![]);
        let result = val.get_optional_array_mut_ref("a").unwrap();
        assert!(result.is_none());
    }

    // ---------------------------------------------------------------
    // get_binary_data / get_optional_binary_data
    // ---------------------------------------------------------------

    #[test]
    fn get_binary_data_works() {
        let val = make_map(vec![("d", Value::Bytes(vec![1, 2]))]);
        let result = val.get_binary_data("d").unwrap();
        assert_eq!(result, BinaryData::new(vec![1, 2]));
    }

    #[test]
    fn get_optional_binary_data_some() {
        let val = make_map(vec![("d", Value::Bytes(vec![3]))]);
        let result = val.get_optional_binary_data("d").unwrap();
        assert_eq!(result, Some(BinaryData::new(vec![3])));
    }

    #[test]
    fn get_optional_binary_data_none() {
        let val = make_map(vec![]);
        let result = val.get_optional_binary_data("d").unwrap();
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // get_bytes / get_optional_bytes
    // ---------------------------------------------------------------

    #[test]
    fn get_bytes_works() {
        let val = make_map(vec![("b", Value::Bytes(vec![10, 11]))]);
        let result = val.get_bytes("b").unwrap();
        assert_eq!(result, vec![10, 11]);
    }

    #[test]
    fn get_optional_bytes_some() {
        let val = make_map(vec![("b", Value::Bytes(vec![12]))]);
        let result = val.get_optional_bytes("b").unwrap();
        assert_eq!(result, Some(vec![12]));
    }

    #[test]
    fn get_optional_bytes_none() {
        let val = make_map(vec![]);
        let result = val.get_optional_bytes("b").unwrap();
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // get_hash256 / get_optional_hash256 / get_identifier / get_optional_identifier
    // ---------------------------------------------------------------

    #[test]
    fn get_hash256_works() {
        let data = [5u8; 32];
        let val = make_map(vec![("h", Value::Bytes32(data))]);
        let result = val.get_hash256("h").unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn get_optional_hash256_some() {
        let data = [6u8; 32];
        let val = make_map(vec![("h", Value::Bytes32(data))]);
        let result = val.get_optional_hash256("h").unwrap();
        assert_eq!(result, Some(data));
    }

    #[test]
    fn get_optional_hash256_none() {
        let val = make_map(vec![]);
        let result = val.get_optional_hash256("h").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_identifier_works() {
        let data = [11u8; 32];
        let val = make_map(vec![("id", Value::Identifier(data))]);
        let result = val.get_identifier("id").unwrap();
        assert_eq!(result, Identifier::new(data));
    }

    #[test]
    fn get_optional_identifier_some() {
        let data = [12u8; 32];
        let val = make_map(vec![("id", Value::Identifier(data))]);
        let result = val.get_optional_identifier("id").unwrap();
        assert_eq!(result, Some(Identifier::new(data)));
    }

    #[test]
    fn get_optional_identifier_none() {
        let val = make_map(vec![]);
        let result = val.get_optional_identifier("id").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_hash256_as_bs58_string_works() {
        let data = [1u8; 32];
        let val = make_map(vec![("h", Value::Bytes32(data))]);
        let result = val.get_hash256_as_bs58_string("h").unwrap();
        let expected = bs58::encode(data).into_string();
        assert_eq!(result, expected);
    }

    // ---------------------------------------------------------------
    // get_string_ref_map / get_optional_string_ref_map
    // ---------------------------------------------------------------

    #[test]
    fn get_string_ref_map_works() {
        let inner = Value::Map(vec![(Value::Text("nested_key".into()), Value::U8(42))]);
        let val = make_map(vec![("m", inner)]);
        let result: BTreeMap<String, &Value> = val.get_string_ref_map("m").unwrap();
        assert_eq!(result.get("nested_key"), Some(&&Value::U8(42)));
    }

    #[test]
    fn get_optional_string_ref_map_some() {
        let inner = Value::Map(vec![(Value::Text("k".into()), Value::Bool(true))]);
        let val = make_map(vec![("m", inner)]);
        let result: Option<BTreeMap<String, &Value>> =
            val.get_optional_string_ref_map("m").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn get_optional_string_ref_map_none_for_missing() {
        let val = make_map(vec![]);
        let result: Option<BTreeMap<String, &Value>> =
            val.get_optional_string_ref_map("m").unwrap();
        assert!(result.is_none());
    }

    // ---------------------------------------------------------------
    // get_string_mut_ref_map / get_optional_string_mut_ref_map
    // ---------------------------------------------------------------

    #[test]
    fn get_string_mut_ref_map_works() {
        let inner = Value::Map(vec![(Value::Text("nested".into()), Value::U8(1))]);
        let mut val = make_map(vec![("m", inner)]);
        let result: BTreeMap<String, &mut Value> = val.get_string_mut_ref_map("m").unwrap();
        assert!(result.contains_key("nested"));
    }

    #[test]
    fn get_optional_string_mut_ref_map_some() {
        let inner = Value::Map(vec![(Value::Text("k".into()), Value::Bool(true))]);
        let mut val = make_map(vec![("m", inner)]);
        let result: Option<BTreeMap<String, &mut Value>> =
            val.get_optional_string_mut_ref_map("m").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn get_optional_string_mut_ref_map_none_for_missing() {
        let mut val = make_map(vec![]);
        let result: Option<BTreeMap<String, &mut Value>> =
            val.get_optional_string_mut_ref_map("m").unwrap();
        assert!(result.is_none());
    }

    // ---------------------------------------------------------------
    // get_optional_bytes_into / get_bytes_into / get_optional_bytes_try_into / get_bytes_try_into
    // ---------------------------------------------------------------

    #[test]
    fn get_optional_bytes_into_some() {
        let val = make_map(vec![("b", Value::Bytes(vec![1, 2, 3]))]);
        let result: Option<BinaryData> = val.get_optional_bytes_into("b").unwrap();
        assert_eq!(result, Some(BinaryData::new(vec![1, 2, 3])));
    }

    #[test]
    fn get_optional_bytes_into_none() {
        let val = make_map(vec![]);
        let result: Option<BinaryData> = val.get_optional_bytes_into("b").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_bytes_into_works() {
        let val = make_map(vec![("b", Value::Bytes(vec![4, 5]))]);
        let result: BinaryData = val.get_bytes_into("b").unwrap();
        assert_eq!(result, BinaryData::new(vec![4, 5]));
    }

    // ---------------------------------------------------------------
    // inner_optional_array_of_strings
    // ---------------------------------------------------------------

    #[test]
    fn inner_optional_array_of_strings_returns_some() {
        let map: Vec<(Value, Value)> = vec![(
            Value::Text("strs".into()),
            Value::Array(vec![Value::Text("a".into()), Value::Text("b".into())]),
        )];
        let result: Option<Vec<String>> = Value::inner_optional_array_of_strings(&map, "strs");
        assert_eq!(result, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn inner_optional_array_of_strings_returns_none_for_missing() {
        let map: Vec<(Value, Value)> = vec![];
        let result: Option<Vec<String>> = Value::inner_optional_array_of_strings(&map, "strs");
        assert_eq!(result, None);
    }

    #[test]
    fn inner_optional_array_of_strings_returns_none_for_non_array() {
        let map: Vec<(Value, Value)> = vec![(Value::Text("strs".into()), Value::U8(1))];
        let result: Option<Vec<String>> = Value::inner_optional_array_of_strings(&map, "strs");
        assert_eq!(result, None);
    }

    // ---------------------------------------------------------------
    // inner_recursive_optional_array_of_strings
    // ---------------------------------------------------------------

    #[test]
    fn inner_recursive_optional_array_of_strings_basic() {
        let map: Vec<(Value, Value)> = vec![(
            Value::Text("required".into()),
            Value::Array(vec![Value::Text("field1".into())]),
        )];
        let result = Value::inner_recursive_optional_array_of_strings(
            &map,
            "".to_string(),
            "properties",
            "required",
        );
        assert!(result.contains("field1"));
    }

    #[test]
    fn inner_recursive_optional_array_of_strings_recursive() {
        let inner_map: Vec<(Value, Value)> = vec![(
            Value::Text("required".into()),
            Value::Array(vec![Value::Text("sub_field".into())]),
        )];
        let map: Vec<(Value, Value)> = vec![
            (
                Value::Text("required".into()),
                Value::Array(vec![Value::Text("top_field".into())]),
            ),
            (
                Value::Text("properties".into()),
                Value::Map(vec![(Value::Text("nested".into()), Value::Map(inner_map))]),
            ),
        ];
        let result = Value::inner_recursive_optional_array_of_strings(
            &map,
            "".to_string(),
            "properties",
            "required",
        );
        assert!(result.contains("top_field"));
        assert!(result.contains("nested.sub_field"));
    }

    // ---------------------------------------------------------------
    // get_from_map / get_optional_from_map / get_mut_from_map / get_optional_mut_from_map
    // ---------------------------------------------------------------

    #[test]
    fn get_from_map_returns_value() {
        let map: Vec<(Value, Value)> = vec![(Value::Text("k".into()), Value::U64(99))];
        let result = Value::get_from_map(&map, "k").unwrap();
        assert_eq!(result, &Value::U64(99));
    }

    #[test]
    fn get_from_map_errors_for_missing() {
        let map: Vec<(Value, Value)> = vec![];
        assert!(Value::get_from_map(&map, "k").is_err());
    }

    #[test]
    fn get_optional_from_map_returns_none_for_null() {
        let map: Vec<(Value, Value)> = vec![(Value::Text("k".into()), Value::Null)];
        let result = Value::get_optional_from_map(&map, "k");
        assert!(result.is_none());
    }

    #[test]
    fn get_optional_from_map_skips_non_text_keys() {
        let map: Vec<(Value, Value)> = vec![(Value::U8(1), Value::U64(99))];
        let result = Value::get_optional_from_map(&map, "1");
        assert!(result.is_none());
    }

    #[test]
    fn get_mut_from_map_works() {
        let mut map: Vec<(Value, Value)> = vec![(Value::Text("k".into()), Value::U64(1))];
        let result = Value::get_mut_from_map(&mut map, "k").unwrap();
        *result = Value::U64(2);
        assert_eq!(map[0].1, Value::U64(2));
    }

    #[test]
    fn get_optional_mut_from_map_returns_none_for_null() {
        let mut map: Vec<(Value, Value)> = vec![(Value::Text("k".into()), Value::Null)];
        let result = Value::get_optional_mut_from_map(&mut map, "k");
        assert!(result.is_none());
    }

    // ---------------------------------------------------------------
    // insert_in_map / insert_in_map_string_value / push_to_map_string_value
    // ---------------------------------------------------------------

    #[test]
    fn insert_in_map_adds_new_entry() {
        let mut map: Vec<(Value, Value)> = vec![];
        Value::insert_in_map(&mut map, "key", Value::U8(1));
        assert_eq!(map.len(), 1);
        assert_eq!(map[0].1, Value::U8(1));
    }

    #[test]
    fn insert_in_map_replaces_existing() {
        let mut map: Vec<(Value, Value)> = vec![(Value::Text("key".into()), Value::U8(1))];
        Value::insert_in_map(&mut map, "key", Value::U8(2));
        assert_eq!(map.len(), 1);
        assert_eq!(map[0].1, Value::U8(2));
    }

    #[test]
    fn insert_in_map_string_value_adds_new() {
        let mut map: Vec<(Value, Value)> = vec![];
        Value::insert_in_map_string_value(&mut map, "b".to_string(), Value::U8(2));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn insert_in_map_string_value_replaces_existing() {
        let mut map: Vec<(Value, Value)> = vec![(Value::Text("b".into()), Value::U8(1))];
        Value::insert_in_map_string_value(&mut map, "b".to_string(), Value::U8(99));
        assert_eq!(map.len(), 1);
        assert_eq!(map[0].1, Value::U8(99));
    }

    #[test]
    fn push_to_map_string_value_adds_new() {
        let mut map: Vec<(Value, Value)> = vec![];
        Value::push_to_map_string_value(&mut map, "z".to_string(), Value::U8(26));
        assert_eq!(map.len(), 1);
        assert_eq!(map[0].1, Value::U8(26));
    }

    #[test]
    fn push_to_map_string_value_replaces_existing() {
        let mut map: Vec<(Value, Value)> = vec![(Value::Text("z".into()), Value::U8(1))];
        Value::push_to_map_string_value(&mut map, "z".to_string(), Value::U8(100));
        assert_eq!(map.len(), 1);
        assert_eq!(map[0].1, Value::U8(100));
    }

    // ---------------------------------------------------------------
    // Error cases: calling map methods on non-map values
    // ---------------------------------------------------------------

    #[test]
    fn get_str_on_non_map_errors() {
        let val = Value::U64(42);
        assert!(val.get_str("key").is_err());
    }

    #[test]
    fn get_bool_on_non_map_errors() {
        let val = Value::U64(42);
        assert!(val.get_bool("key").is_err());
    }

    #[test]
    fn get_bytes_on_non_map_errors() {
        let val = Value::U64(42);
        assert!(val.get_bytes("key").is_err());
    }

    #[test]
    fn remove_on_non_map_errors() {
        let mut val = Value::U64(42);
        assert!(val.remove("key").is_err());
    }

    #[test]
    fn set_value_on_non_map_errors() {
        let mut val = Value::U64(42);
        assert!(val.set_value("key", Value::Bool(true)).is_err());
    }

    #[test]
    fn insert_on_non_map_errors() {
        let mut val = Value::U64(42);
        assert!(val.insert("key".to_string(), Value::Bool(true)).is_err());
    }
}
