use crate::{Error, Value};
use indexmap::IndexMap;
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub type ValueMap = Vec<(Value, Value)>;

pub trait ValueMapHelper {
    fn sort_by_keys(&mut self);
    fn sort_by_keys_and_inner_maps(&mut self);
    fn sort_by_lexicographical_byte_ordering_keys(&mut self);
    fn sort_by_lexicographical_byte_ordering_keys_and_inner_maps(&mut self);
    fn get_key(&self, search_key: &str) -> Result<&Value, Error>;
    fn get_optional_key(&self, key: &str) -> Option<&Value>;
    fn get_key_mut(&mut self, search_key: &str) -> Result<&mut Value, Error>;
    fn get_optional_key_mut(&mut self, key: &str) -> Option<&mut Value>;
    fn get_key_mut_or_insert(&mut self, key: &str, value: Value) -> &mut Value;
    fn get_key_by_value_mut_or_insert(&mut self, search_key: &Value, value: Value) -> &mut Value;
    fn insert_string_key_value(&mut self, key: String, value: Value);
    fn remove_key(&mut self, search_key: &str) -> Result<Value, Error>;
    fn remove_optional_key(&mut self, key: &str) -> Option<Value>;
    fn remove_optional_key_if_null(&mut self, search_key: &str);
    fn remove_optional_key_if_empty_array(&mut self, search_key: &str);
    fn remove_optional_key_value(&mut self, search_key_value: &Value) -> Option<Value>;
}

impl ValueMapHelper for ValueMap {
    fn sort_by_keys(&mut self) {
        self.sort_by(|(key1, _), (key2, _)| key1.partial_cmp(key2).unwrap_or(Ordering::Less));
    }

    fn sort_by_keys_and_inner_maps(&mut self) {
        self.sort_by_keys();
        self.iter_mut().for_each(|(_, v)| {
            if let Value::Map(m) = v {
                m.sort_by_keys_and_inner_maps()
            }
        });
    }

    fn sort_by_lexicographical_byte_ordering_keys(&mut self) {
        self.sort_by(|(key1, _), (key2, _)| {
            if key1.is_text() && key2.is_text() {
                let key1 = key1.to_text().unwrap();
                let key2 = key2.to_text().unwrap();
                match key1.len().cmp(&key2.len()) {
                    Ordering::Less => Ordering::Less,
                    Ordering::Equal => key1.cmp(&key2),
                    Ordering::Greater => Ordering::Greater,
                }
            } else {
                key1.partial_cmp(key2).unwrap_or(Ordering::Less)
            }
        })
    }

    fn sort_by_lexicographical_byte_ordering_keys_and_inner_maps(&mut self) {
        self.sort_by_lexicographical_byte_ordering_keys();
        self.iter_mut().for_each(|(_, v)| {
            if let Value::Map(m) = v {
                m.sort_by_lexicographical_byte_ordering_keys_and_inner_maps()
            }
        });
    }

    fn get_key(&self, search_key: &str) -> Result<&Value, Error> {
        self.get_optional_key(search_key)
            .ok_or(Error::StructureError(format!(
                "required property not found {search_key}"
            )))
    }

    fn get_optional_key(&self, search_key: &str) -> Option<&Value> {
        self.iter().find_map(|(key, value)| {
            if let Value::Text(text) = key {
                if text == search_key {
                    Some(value)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    fn get_key_mut(&mut self, search_key: &str) -> Result<&mut Value, Error> {
        self.get_optional_key_mut(search_key)
            .ok_or(Error::StructureError(format!(
                "{search_key} not found, but was required"
            )))
    }

    fn get_optional_key_mut(&mut self, search_key: &str) -> Option<&mut Value> {
        self.iter_mut().find_map(|(key, value)| {
            if let Value::Text(text) = key {
                if text == search_key {
                    Some(value)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    fn get_key_mut_or_insert(&mut self, search_key: &str, value: Value) -> &mut Value {
        let found = self.iter().position(|(key, _)| {
            if let Value::Text(text) = key {
                text == search_key
            } else {
                false
            }
        });
        match found {
            None => {
                self.push((Value::Text(search_key.to_string()), value));
                let (_, value) = self.last_mut().unwrap();
                value
            }
            Some(pos) => {
                let (_, value) = self.get_mut(pos).unwrap();
                value
            }
        }
    }

    fn get_key_by_value_mut_or_insert(&mut self, search_key: &Value, value: Value) -> &mut Value {
        let found = self.iter().position(|(key, _)| search_key == key);
        match found {
            None => {
                self.push((search_key.clone(), value));
                let (_, value) = self.last_mut().unwrap();
                value
            }
            Some(pos) => {
                let (_, value) = self.get_mut(pos).unwrap();
                value
            }
        }
    }

    fn insert_string_key_value(&mut self, key: String, value: Value) {
        self.push((key.into(), value))
    }

    fn remove_key(&mut self, search_key: &str) -> Result<Value, Error> {
        self.iter()
            .position(|(key, _)| {
                if let Value::Text(text) = key {
                    text == search_key
                } else {
                    false
                }
            })
            .map(|pos| self.remove(pos).1)
            .ok_or(Error::StructureError(format!(
                "trying to remove a key {} from a ValueMap that was not found",
                search_key
            )))
    }

    fn remove_optional_key(&mut self, search_key: &str) -> Option<Value> {
        self.iter()
            .position(|(key, _)| {
                if let Value::Text(text) = key {
                    text == search_key
                } else {
                    false
                }
            })
            .map(|pos| self.remove(pos).1)
    }

    fn remove_optional_key_if_null(&mut self, search_key: &str) {
        self.iter()
            .position(|(key, value)| {
                if let Value::Text(text) = key {
                    if text == search_key {
                        value.is_null()
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .map(|pos| self.remove(pos).1);
    }

    fn remove_optional_key_if_empty_array(&mut self, search_key: &str) {
        self.iter()
            .position(|(key, value)| {
                if let Value::Text(text) = key {
                    if text == search_key {
                        if let Some(v) = value.as_array() {
                            v.is_empty()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .map(|pos| self.remove(pos).1);
    }

    fn remove_optional_key_value(&mut self, search_key_value: &Value) -> Option<Value> {
        self.iter()
            .position(|(key, _)| search_key_value == key)
            .map(|pos| self.remove(pos).1)
    }
}

impl Value {
    /// If the `Value` is a `Map`, returns a the associated `BTreeMap<String, Value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use std::collections::BTreeMap;
    /// # use platform_value::{Error, Value};
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("key")), Value::Float(18.)),
    ///     ]
    /// );
    /// assert_eq!(value.into_btree_string_map(), Ok(BTreeMap::from([(String::from("key"), Value::Float(18.))])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_btree_string_map(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn into_btree_string_map(self) -> Result<BTreeMap<String, Value>, Error> {
        Self::map_into_btree_string_map(self.into_map()?)
    }

    /// If the `Value` is a `Map`, returns a the associated `BTreeMap<String, Value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use std::collections::BTreeMap;
    /// # use platform_value::{Error, Value};
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("key")), Value::Float(18.)),
    ///     ]
    /// );
    /// assert_eq!(value.to_btree_ref_string_map(), Ok(BTreeMap::from([(String::from("key"), &Value::Float(18.))])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_btree_ref_string_map(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn to_btree_ref_string_map(&self) -> Result<BTreeMap<String, &Value>, Error> {
        Self::map_ref_into_btree_string_map(self.to_map_ref()?)
    }

    /// If the `Value` is a `Map`, returns a the associated `BTreeMap<String, Value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use std::collections::BTreeMap;
    /// # use platform_value::{Error, Value};
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("key")), Value::Float(18.)),
    ///     ]
    /// );
    /// assert_eq!(value.to_ref_string_map::<BTreeMap<_,_>>(), Ok(BTreeMap::from([(String::from("key"), &Value::Float(18.))])));
    ///
    /// assert_eq!(value.to_ref_string_map::<Vec<(_,_)>>(), Ok(vec![(String::from("key"), &Value::Float(18.))]));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_ref_string_map::<Vec<(_,_)>>(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn to_ref_string_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
    ) -> Result<I, Error> {
        Self::map_ref_into_string_map(self.to_map_ref()?)
    }

    /// If the `Value` is a `Map`, returns a the associated `BTreeMap<String, Value>` data as `Ok`.
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    ///
    /// ```
    /// # use std::collections::BTreeMap;
    /// # use platform_value::{Error, Value};
    /// #
    /// let mut value = Value::Map(
    ///     vec![
    ///         (Value::Text(String::from("key")), Value::Float(18.)),
    ///     ]
    /// );
    /// assert_eq!(value.to_ref_string_map_mut::<BTreeMap<_,_>>(), Ok(BTreeMap::from([(String::from("key"), &mut Value::Float(18.))])));
    ///
    /// assert_eq!(value.to_ref_string_map_mut::<Vec<(_,_)>>(), Ok(vec![(String::from("key"), &mut Value::Float(18.))]));
    ///
    /// let mut value = Value::Bool(true);
    /// assert_eq!(value.to_ref_string_map_mut::<Vec<(_,_)>>(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn to_ref_string_map_mut<'a, I: FromIterator<(String, &'a mut Value)>>(
        &'a mut self,
    ) -> Result<I, Error> {
        Self::map_mut_ref_into_string_map(self.as_map_mut_ref()?)
    }

    /// Takes a ValueMap which is a `Vec<(Value, Value)>`
    /// Returns a BTreeMap<String, Value> as long as each Key is a String
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    pub fn map_into_btree_string_map(map: ValueMap) -> Result<BTreeMap<String, Value>, Error> {
        map.into_iter()
            .map(|(key, value)| {
                let key = key
                    .into_text()
                    .map_err(|_| Error::StructureError("expected key to be string".to_string()))?;
                Ok((key, value))
            })
            .collect::<Result<BTreeMap<String, Value>, Error>>()
    }

    /// Takes a ref to a ValueMap which is a `&Vec<(Value, Value)>`
    /// Returns a BTreeMap<String, &Value> as long as each Key is a String
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    pub fn map_ref_into_btree_string_map(
        map: &ValueMap,
    ) -> Result<BTreeMap<String, &Value>, Error> {
        map.iter()
            .map(|(key, value)| {
                let key = key
                    .to_text()
                    .map_err(|_| Error::StructureError("expected key to be string".to_string()))?;
                Ok((key, value))
            })
            .collect::<Result<BTreeMap<String, &Value>, Error>>()
    }

    /// Takes a ref to a ValueMap which is a `&Vec<(Value, Value)>`
    /// Also takes a sort_key
    /// Returns a IndexMap<String, &Value> as long as each Key is a String
    /// The index map is in the order sorted by the sort key
    /// The type T is the type of the value of the sort key
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    pub fn map_ref_into_indexed_string_map<'a, 'b, T>(
        map: &'a ValueMap,
        sort_key: &'b str,
    ) -> Result<IndexMap<String, &'a Value>, Error>
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
        // Check if the sort key exists in all values
        for (_, value) in map.iter() {
            value.get_integer::<T>(sort_key)?;
        }

        let mut sorted_map: Vec<_> = map.iter().collect();

        sorted_map.sort_by(|(_, value_1), (_, value_2)| {
            let pos_1: T = value_1.get_integer(sort_key).expect("expected sort key");
            let pos_2: T = value_2.get_integer(sort_key).expect("expected sort key");
            pos_1.cmp(&pos_2)
        });

        sorted_map
            .into_iter()
            .map(|(key, value)| {
                let key = key
                    .to_text()
                    .map_err(|_| Error::StructureError("expected key to be string".to_string()))?;
                Ok((key, value))
            })
            .collect::<Result<IndexMap<String, &Value>, Error>>()
    }

    /// Takes a ref to a ValueMap which is a `&Vec<(Value, Value)>`
    /// Returns a BTreeMap<String, &Value> as long as each Key is a String
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    pub fn map_ref_into_string_map<'a, I: FromIterator<(String, &'a Value)>>(
        map: &'a ValueMap,
    ) -> Result<I, Error> {
        map.iter()
            .map(|(key, value)| {
                let key = key
                    .to_text()
                    .map_err(|_| Error::StructureError("expected key to be string".to_string()))?;
                Ok((key, value))
            })
            .collect::<Result<I, Error>>()
    }

    /// Takes a ref to a ValueMap which is a `&Vec<(Value, Value)>`
    /// Returns a BTreeMap<String, &Value> as long as each Key is a String
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    pub fn map_mut_ref_into_string_map<'a, I: FromIterator<(String, &'a mut Value)>>(
        map: &'a mut ValueMap,
    ) -> Result<I, Error> {
        map.iter_mut()
            .map(|(key, value)| {
                let key = key
                    .to_text()
                    .map_err(|_| Error::StructureError("expected key to be string".to_string()))?;
                Ok((key, value))
            })
            .collect::<Result<I, Error>>()
    }
}
