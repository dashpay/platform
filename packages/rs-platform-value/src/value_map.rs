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
    fn from_btree_map<K: Into<Value> + Ord, V: Into<Value>>(btree_map: BTreeMap<K, V>) -> Self;
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
    fn from_btree_map<K: Into<Value> + Ord, V: Into<Value>>(btree_map: BTreeMap<K, V>) -> Self {
        btree_map
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    fn text(s: &str) -> Value {
        Value::Text(s.to_string())
    }

    fn make_map(pairs: &[(&str, Value)]) -> ValueMap {
        pairs.iter().map(|(k, v)| (text(k), v.clone())).collect()
    }

    // ---------------------------------------------------------------
    // sort_by_keys
    // ---------------------------------------------------------------

    #[test]
    fn sort_by_keys_mixed_text_keys() {
        let mut map = make_map(&[
            ("c", Value::U32(3)),
            ("a", Value::U32(1)),
            ("b", Value::U32(2)),
        ]);
        map.sort_by_keys();
        let keys: Vec<_> = map.iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(keys, vec![text("a"), text("b"), text("c")]);
    }

    #[test]
    fn sort_by_keys_mixed_types() {
        // Integer keys should sort before text keys via PartialOrd on Value
        let mut map: ValueMap = vec![
            (text("z"), Value::U32(1)),
            (Value::U32(5), Value::U32(2)),
            (text("a"), Value::U32(3)),
        ];
        map.sort_by_keys();
        // U32(5) < Text("a") < Text("z") by Value's PartialOrd (enum variant order)
        assert_eq!(map[0].0, Value::U32(5));
        assert_eq!(map[1].0, text("a"));
        assert_eq!(map[2].0, text("z"));
    }

    // ---------------------------------------------------------------
    // sort_by_lexicographical_byte_ordering_keys
    // ---------------------------------------------------------------

    #[test]
    fn sort_by_lexicographical_byte_ordering_shorter_first() {
        // "ab" (len 2) should come before "abc" (len 3)
        let mut map = make_map(&[
            ("abc", Value::U32(1)),
            ("ab", Value::U32(2)),
            ("a", Value::U32(3)),
        ]);
        map.sort_by_lexicographical_byte_ordering_keys();
        let keys: Vec<_> = map.iter().map(|(k, _)| k.to_text().unwrap()).collect();
        assert_eq!(keys, vec!["a", "ab", "abc"]);
    }

    #[test]
    fn sort_by_lexicographical_byte_ordering_same_length_alphabetical() {
        let mut map = make_map(&[
            ("cb", Value::U32(1)),
            ("ab", Value::U32(2)),
            ("bb", Value::U32(3)),
        ]);
        map.sort_by_lexicographical_byte_ordering_keys();
        let keys: Vec<_> = map.iter().map(|(k, _)| k.to_text().unwrap()).collect();
        assert_eq!(keys, vec!["ab", "bb", "cb"]);
    }

    #[test]
    fn sort_by_lexicographical_byte_ordering_non_text_keys_uses_partial_cmp() {
        let mut map: ValueMap = vec![(Value::U32(10), Value::Null), (Value::U32(2), Value::Null)];
        map.sort_by_lexicographical_byte_ordering_keys();
        assert_eq!(map[0].0, Value::U32(2));
        assert_eq!(map[1].0, Value::U32(10));
    }

    // ---------------------------------------------------------------
    // get_key_mut_or_insert
    // ---------------------------------------------------------------

    #[test]
    fn get_key_mut_or_insert_inserts_new() {
        let mut map = make_map(&[("a", Value::U32(1))]);
        let val = map.get_key_mut_or_insert("b", Value::U32(99));
        assert_eq!(*val, Value::U32(99));
        // Mutate the returned reference
        *val = Value::U32(100);
        assert_eq!(map.get_optional_key("b"), Some(&Value::U32(100)));
    }

    #[test]
    fn get_key_mut_or_insert_returns_existing() {
        let mut map = make_map(&[("a", Value::U32(1))]);
        let val = map.get_key_mut_or_insert("a", Value::U32(99));
        // Should return existing value, not the default
        assert_eq!(*val, Value::U32(1));
    }

    #[test]
    fn get_key_mut_or_insert_existing_is_mutable() {
        let mut map = make_map(&[("a", Value::U32(1))]);
        let val = map.get_key_mut_or_insert("a", Value::U32(99));
        *val = Value::U32(42);
        assert_eq!(map.get_optional_key("a"), Some(&Value::U32(42)));
    }

    // ---------------------------------------------------------------
    // remove_optional_key_if_null
    // ---------------------------------------------------------------

    #[test]
    fn remove_optional_key_if_null_removes_null() {
        let mut map = make_map(&[("a", Value::Null), ("b", Value::U32(2))]);
        map.remove_optional_key_if_null("a");
        assert_eq!(map.get_optional_key("a"), None);
        assert_eq!(map.get_optional_key("b"), Some(&Value::U32(2)));
    }

    #[test]
    fn remove_optional_key_if_null_keeps_non_null() {
        let mut map = make_map(&[("a", Value::U32(1))]);
        map.remove_optional_key_if_null("a");
        assert_eq!(map.get_optional_key("a"), Some(&Value::U32(1)));
    }

    #[test]
    fn remove_optional_key_if_null_missing_key_is_noop() {
        let mut map = make_map(&[("a", Value::U32(1))]);
        map.remove_optional_key_if_null("missing");
        assert_eq!(map.len(), 1);
    }

    // ---------------------------------------------------------------
    // remove_optional_key_if_empty_array
    // ---------------------------------------------------------------

    #[test]
    fn remove_optional_key_if_empty_array_removes_empty() {
        let mut map = make_map(&[("a", Value::Array(vec![])), ("b", Value::U32(1))]);
        map.remove_optional_key_if_empty_array("a");
        assert_eq!(map.get_optional_key("a"), None);
        assert_eq!(map.get_optional_key("b"), Some(&Value::U32(1)));
    }

    #[test]
    fn remove_optional_key_if_empty_array_keeps_non_empty() {
        let mut map = make_map(&[("a", Value::Array(vec![Value::U32(1)]))]);
        map.remove_optional_key_if_empty_array("a");
        assert!(map.get_optional_key("a").is_some());
    }

    #[test]
    fn remove_optional_key_if_empty_array_keeps_non_array() {
        let mut map = make_map(&[("a", Value::U32(42))]);
        map.remove_optional_key_if_empty_array("a");
        assert_eq!(map.get_optional_key("a"), Some(&Value::U32(42)));
    }

    #[test]
    fn remove_optional_key_if_empty_array_missing_key_is_noop() {
        let mut map = make_map(&[("a", Value::U32(1))]);
        map.remove_optional_key_if_empty_array("missing");
        assert_eq!(map.len(), 1);
    }

    // ---------------------------------------------------------------
    // into_btree_string_map
    // ---------------------------------------------------------------

    #[test]
    fn into_btree_string_map_valid_conversion() {
        let val = Value::Map(make_map(&[("b", Value::U32(2)), ("a", Value::U32(1))]));
        let btree = val.into_btree_string_map().unwrap();
        assert_eq!(btree.get("a"), Some(&Value::U32(1)));
        assert_eq!(btree.get("b"), Some(&Value::U32(2)));
        // BTreeMap should be sorted by key
        let keys: Vec<_> = btree.keys().collect();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn into_btree_string_map_error_on_non_string_keys() {
        let val = Value::Map(vec![(Value::U32(1), Value::U32(2))]);
        let result = val.into_btree_string_map();
        assert!(result.is_err());
    }

    #[test]
    fn into_btree_string_map_error_on_non_map() {
        let val = Value::Bool(true);
        let result = val.into_btree_string_map();
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // map_ref_into_indexed_string_map
    // ---------------------------------------------------------------

    #[test]
    fn map_ref_into_indexed_string_map_sorts_by_integer_key() {
        let map: ValueMap = vec![
            (
                text("second"),
                Value::Map(make_map(&[("pos", Value::U32(2))])),
            ),
            (
                text("first"),
                Value::Map(make_map(&[("pos", Value::U32(1))])),
            ),
            (
                text("third"),
                Value::Map(make_map(&[("pos", Value::U32(3))])),
            ),
        ];
        let indexed = Value::map_ref_into_indexed_string_map::<u32>(&map, "pos").unwrap();
        let keys: Vec<_> = indexed.keys().collect();
        assert_eq!(keys, vec!["first", "second", "third"]);
    }

    #[test]
    fn map_ref_into_indexed_string_map_error_missing_sort_key() {
        let map: ValueMap = vec![(
            text("item"),
            Value::Map(make_map(&[("other", Value::U32(1))])),
        )];
        let result = Value::map_ref_into_indexed_string_map::<u32>(&map, "pos");
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // get_key / get_optional_key
    // ---------------------------------------------------------------

    #[test]
    fn get_key_found() {
        let map = make_map(&[("x", Value::U32(42))]);
        let val = map.get_key("x").unwrap();
        assert_eq!(*val, Value::U32(42));
    }

    #[test]
    fn get_key_not_found_errors() {
        let map = make_map(&[("x", Value::U32(42))]);
        assert!(map.get_key("y").is_err());
    }

    #[test]
    fn get_optional_key_none_for_missing() {
        let map = make_map(&[("x", Value::U32(42))]);
        assert_eq!(map.get_optional_key("y"), None);
    }

    #[test]
    fn get_optional_key_ignores_non_text_keys() {
        let map: ValueMap = vec![(Value::U32(1), Value::U32(2))];
        assert_eq!(map.get_optional_key("1"), None);
    }

    // ---------------------------------------------------------------
    // remove_key / remove_optional_key
    // ---------------------------------------------------------------

    #[test]
    fn remove_key_success() {
        let mut map = make_map(&[("a", Value::U32(1)), ("b", Value::U32(2))]);
        let removed = map.remove_key("a").unwrap();
        assert_eq!(removed, Value::U32(1));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn remove_key_not_found_errors() {
        let mut map = make_map(&[("a", Value::U32(1))]);
        assert!(map.remove_key("missing").is_err());
    }

    #[test]
    fn remove_optional_key_returns_none_for_missing() {
        let mut map = make_map(&[("a", Value::U32(1))]);
        assert_eq!(map.remove_optional_key("missing"), None);
    }

    #[test]
    fn remove_optional_key_returns_value() {
        let mut map = make_map(&[("a", Value::U32(1))]);
        assert_eq!(map.remove_optional_key("a"), Some(Value::U32(1)));
        assert!(map.is_empty());
    }

    // ---------------------------------------------------------------
    // remove_optional_key_value
    // ---------------------------------------------------------------

    #[test]
    fn remove_optional_key_value_by_value_key() {
        let mut map: ValueMap = vec![
            (Value::U32(10), Value::Bool(true)),
            (text("x"), Value::Bool(false)),
        ];
        let removed = map.remove_optional_key_value(&Value::U32(10));
        assert_eq!(removed, Some(Value::Bool(true)));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn remove_optional_key_value_not_found() {
        let mut map = make_map(&[("a", Value::U32(1))]);
        assert_eq!(map.remove_optional_key_value(&Value::U32(99)), None);
    }

    // ---------------------------------------------------------------
    // insert_string_key_value
    // ---------------------------------------------------------------

    #[test]
    fn insert_string_key_value_appends() {
        let mut map: ValueMap = vec![];
        map.insert_string_key_value("hello".to_string(), Value::Bool(true));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get_optional_key("hello"), Some(&Value::Bool(true)));
    }

    // ---------------------------------------------------------------
    // from_btree_map
    // ---------------------------------------------------------------

    #[test]
    fn from_btree_map_preserves_entries() {
        let mut btree = BTreeMap::new();
        btree.insert("b".to_string(), Value::U32(2));
        btree.insert("a".to_string(), Value::U32(1));
        let map = ValueMap::from_btree_map(btree);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get_optional_key("a"), Some(&Value::U32(1)));
        assert_eq!(map.get_optional_key("b"), Some(&Value::U32(2)));
    }

    // ---------------------------------------------------------------
    // get_key_by_value_mut_or_insert
    // ---------------------------------------------------------------

    #[test]
    fn get_key_by_value_mut_or_insert_inserts_new() {
        let mut map: ValueMap = vec![];
        let val = map.get_key_by_value_mut_or_insert(&Value::U32(42), Value::Bool(true));
        assert_eq!(*val, Value::Bool(true));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn get_key_by_value_mut_or_insert_returns_existing() {
        let mut map: ValueMap = vec![(Value::U32(42), Value::Bool(false))];
        let val = map.get_key_by_value_mut_or_insert(&Value::U32(42), Value::Bool(true));
        assert_eq!(*val, Value::Bool(false));
    }

    // ---------------------------------------------------------------
    // sort_by_keys_and_inner_maps
    // ---------------------------------------------------------------

    #[test]
    fn sort_by_keys_and_inner_maps_sorts_recursively() {
        let inner = make_map(&[("z", Value::U32(1)), ("a", Value::U32(2))]);
        let mut map = make_map(&[("b", Value::Map(inner)), ("a", Value::U32(3))]);
        map.sort_by_keys_and_inner_maps();
        // Outer keys should be sorted
        assert_eq!(map[0].0, text("a"));
        assert_eq!(map[1].0, text("b"));
        // Inner map should also be sorted
        if let Value::Map(ref inner) = map[1].1 {
            assert_eq!(inner[0].0, text("a"));
            assert_eq!(inner[1].0, text("z"));
        } else {
            panic!("expected inner map");
        }
    }

    // ---------------------------------------------------------------
    // to_btree_ref_string_map
    // ---------------------------------------------------------------

    #[test]
    fn to_btree_ref_string_map_valid() {
        let val = Value::Map(make_map(&[("x", Value::U32(10))]));
        let btree = val.to_btree_ref_string_map().unwrap();
        assert_eq!(btree.get("x"), Some(&&Value::U32(10)));
    }

    #[test]
    fn to_btree_ref_string_map_error_on_non_map() {
        let val = Value::U32(1);
        assert!(val.to_btree_ref_string_map().is_err());
    }

    #[test]
    fn to_btree_ref_string_map_error_on_non_string_key() {
        let val = Value::Map(vec![(Value::U32(1), Value::U32(2))]);
        assert!(val.to_btree_ref_string_map().is_err());
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
    pub fn map_ref_into_indexed_string_map<'a, T>(
        map: &'a ValueMap,
        sort_key: &str,
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
