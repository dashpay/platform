use crate::{Error, Value};
use std::collections::{BTreeMap};

pub type ValueMap = Vec<(Value, Value)>;

pub trait ValueMapHelper {
    fn get_key(&self, key: &str) -> Option<&Value>;
    fn get_key_mut(&mut self, key: &str) -> Option<&mut Value>;
    fn get_key_mut_or_insert(&mut self, key: &str, value: Value) -> &mut Value;
    fn remove_key(&mut self, key: &str) -> Option<Value>;
}

impl ValueMapHelper for ValueMap {
    fn get_key(&self, search_key: &str) -> Option<&Value> {
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

    fn get_key_mut(&mut self, search_key: &str) -> Option<&mut Value> {
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
                if text == search_key {
                    true
                } else {
                    false
                }
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

    fn remove_key(&mut self, search_key: &str) -> Option<Value> {
        self.iter()
            .position(|(key, _)| {
                if let Value::Text(text) = key {
                    if text == search_key {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
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
    /// assert_eq!(value.into_btree_map(), Ok(BTreeMap::from([(String::from("key"), Value::Float(18.))])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.into_btree_map(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn into_btree_map(self) -> Result<BTreeMap<String, Value>, Error> {
        Self::map_into_btree_map(self.into_map()?)
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
    /// assert_eq!(value.to_btree_ref_map(), Ok(BTreeMap::from([(String::from("key"), &Value::Float(18.))])));
    ///
    /// let value = Value::Bool(true);
    /// assert_eq!(value.to_btree_ref_map(), Err(Error::StructureError("value is not a map".to_string())))
    /// ```
    pub fn to_btree_ref_map(&self) -> Result<BTreeMap<String, &Value>, Error> {
        Self::map_ref_into_btree_map(self.to_map_ref()?)
    }

    /// Takes a ValueMap which is a `Vec<(Value, Value)>`
    /// Returns a BTreeMap<String, Value> as long as each Key is a String
    /// Returns `Err(Error::Structure("reason"))` otherwise.
    pub fn map_into_btree_map(map: ValueMap) -> Result<BTreeMap<String, Value>, Error> {
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
    pub fn map_ref_into_btree_map(map: &ValueMap) -> Result<BTreeMap<String, &Value>, Error> {
        map.iter()
            .map(|(key, value)| {
                let key = key
                    .to_text()
                    .map_err(|_| Error::StructureError("expected key to be string".to_string()))?;
                Ok((key, value))
            })
            .collect::<Result<BTreeMap<String, &Value>, Error>>()
    }
}
