use std::{
    cmp::Ordering,
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
};

use ciborium::value::Value as CborValue;

use serde::Serialize;

use crate::ProtocolError;

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

    pub fn replace_path(&mut self, path: &str, from: FieldType, to: FieldType) -> Option<()> {
        let cbor_value = self.get_path_mut(path)?;
        let replace_with = convert_to(cbor_value, from, to)?;

        *cbor_value = replace_with;

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
    use std::collections::BTreeMap;
    use std::convert::TryFrom;
    use std::convert::TryInto;

    use crate::util::cbor_value::{ReplacePaths, ValuesCollection};

    use super::{value_to_bytes, value_to_hash, CborCanonicalMap, CborValue, FieldType};
    use ciborium::cbor;

    // ------- Existing tests -------

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

    // ------- New coverage tests -------

    // CborCanonicalMap construction and basic operations

    #[test]
    fn new_creates_empty_map() {
        let map = CborCanonicalMap::new();
        let value = map.to_value_unsorted();
        assert_eq!(value, CborValue::Map(vec![]));
    }

    #[test]
    fn default_creates_empty_map() {
        let map = CborCanonicalMap::default();
        let value = map.to_value_unsorted();
        assert_eq!(value, CborValue::Map(vec![]));
    }

    #[test]
    fn insert_adds_key_value_pair() {
        let mut map = CborCanonicalMap::new();
        map.insert("hello", CborValue::Text("world".to_string()));

        let val = ValuesCollection::get(&map, &CborValue::Text("hello".to_string()));
        assert_eq!(val, Some(&CborValue::Text("world".to_string())));
    }

    #[test]
    fn insert_multiple_keys() {
        let mut map = CborCanonicalMap::new();
        map.insert("a", CborValue::Integer(1.into()));
        map.insert("b", CborValue::Integer(2.into()));
        map.insert("c", CborValue::Integer(3.into()));

        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("a".to_string())),
            Some(&CborValue::Integer(1.into()))
        );
        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("b".to_string())),
            Some(&CborValue::Integer(2.into()))
        );
        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("c".to_string())),
            Some(&CborValue::Integer(3.into()))
        );
    }

    #[test]
    fn remove_existing_key() {
        let mut map = CborCanonicalMap::new();
        map.insert("key1", CborValue::Bool(true));
        map.insert("key2", CborValue::Bool(false));

        map.remove("key1");

        assert!(ValuesCollection::get(&map, &CborValue::Text("key1".to_string())).is_none());
        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("key2".to_string())),
            Some(&CborValue::Bool(false))
        );
    }

    #[test]
    fn remove_nonexistent_key_is_noop() {
        let mut map = CborCanonicalMap::new();
        map.insert("key1", CborValue::Bool(true));

        // Should not panic or change anything
        map.remove("nonexistent");

        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("key1".to_string())),
            Some(&CborValue::Bool(true))
        );
    }

    #[test]
    fn get_mut_returns_mutable_reference() {
        let mut map = CborCanonicalMap::new();
        map.insert("key", CborValue::Integer(10.into()));

        let val = map.get_mut(&CborValue::Text("key".to_string()));
        assert!(val.is_some());
        *val.unwrap() = CborValue::Integer(20.into());

        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("key".to_string())),
            Some(&CborValue::Integer(20.into()))
        );
    }

    #[test]
    fn get_mut_returns_none_for_missing_key() {
        let mut map = CborCanonicalMap::new();
        assert!(map
            .get_mut(&CborValue::Text("missing".to_string()))
            .is_none());
    }

    #[test]
    fn set_replaces_existing_value() {
        let mut map = CborCanonicalMap::new();
        map.insert("key", CborValue::Integer(1.into()));

        let result = map.set(
            &CborValue::Text("key".to_string()),
            CborValue::Integer(99.into()),
        );
        assert!(result.is_some());

        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("key".to_string())),
            Some(&CborValue::Integer(99.into()))
        );
    }

    #[test]
    fn set_returns_none_for_missing_key() {
        let mut map = CborCanonicalMap::new();

        let result = map.set(
            &CborValue::Text("missing".to_string()),
            CborValue::Integer(1.into()),
        );
        assert!(result.is_none());
    }

    #[test]
    fn from_vector_creates_map() {
        let vec = vec![
            (
                CborValue::Text("a".to_string()),
                CborValue::Integer(1.into()),
            ),
            (
                CborValue::Text("b".to_string()),
                CborValue::Integer(2.into()),
            ),
        ];

        let map = CborCanonicalMap::from_vector(vec);

        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("a".to_string())),
            Some(&CborValue::Integer(1.into()))
        );
        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("b".to_string())),
            Some(&CborValue::Integer(2.into()))
        );
    }

    #[test]
    fn from_serializable_with_btreemap() {
        let mut btree = BTreeMap::new();
        btree.insert("name".to_string(), "test".to_string());

        let map =
            CborCanonicalMap::from_serializable(&btree).expect("should serialize from BTreeMap");

        assert!(ValuesCollection::get(&map, &CborValue::Text("name".to_string())).is_some());
    }

    #[test]
    fn from_serializable_with_non_map_value_fails() {
        // A plain string serializes as CborValue::Text, not a Map
        let result = CborCanonicalMap::from_serializable(&"just a string");
        assert!(result.is_err());
    }

    // Canonical sorting

    #[test]
    fn sort_canonical_orders_by_key_length_then_lexicographic() {
        let mut map = CborCanonicalMap::new();
        // Longer key first
        map.insert("beta", CborValue::Integer(2.into()));
        map.insert("a", CborValue::Integer(1.into()));
        map.insert("cc", CborValue::Integer(3.into()));
        map.insert("bb", CborValue::Integer(4.into()));

        map.sort_canonical();

        let sorted = map.to_value_unsorted();
        if let CborValue::Map(pairs) = sorted {
            let keys: Vec<&str> = pairs.iter().map(|(k, _)| k.as_text().unwrap()).collect();
            // "a" (len 1) < "bb" (len 2) < "cc" (len 2, but bb < cc) < "beta" (len 4)
            assert_eq!(keys, vec!["a", "bb", "cc", "beta"]);
        } else {
            panic!("Expected map");
        }
    }

    #[test]
    fn sort_canonical_recursively_sorts_nested_maps() {
        let mut map = CborCanonicalMap::new();
        // Create a nested map with unsorted keys
        let nested = CborValue::Map(vec![
            (
                CborValue::Text("zz".to_string()),
                CborValue::Integer(1.into()),
            ),
            (
                CborValue::Text("a".to_string()),
                CborValue::Integer(2.into()),
            ),
        ]);
        map.insert("outer", nested);

        map.sort_canonical();

        let sorted = map.to_value_unsorted();
        if let CborValue::Map(pairs) = sorted {
            if let CborValue::Map(inner_pairs) = &pairs[0].1 {
                let keys: Vec<&str> = inner_pairs
                    .iter()
                    .map(|(k, _)| k.as_text().unwrap())
                    .collect();
                // "a" (len 1) should come before "zz" (len 2)
                assert_eq!(keys, vec!["a", "zz"]);
            } else {
                panic!("Expected nested map");
            }
        }
    }

    #[test]
    fn sort_canonical_recursively_sorts_maps_inside_arrays() {
        let mut map = CborCanonicalMap::new();
        let nested_map_in_array = CborValue::Array(vec![CborValue::Map(vec![
            (
                CborValue::Text("zz".to_string()),
                CborValue::Integer(1.into()),
            ),
            (
                CborValue::Text("a".to_string()),
                CborValue::Integer(2.into()),
            ),
        ])]);
        map.insert("items", nested_map_in_array);

        map.sort_canonical();

        let sorted = map.to_value_unsorted();
        if let CborValue::Map(pairs) = sorted {
            if let CborValue::Array(arr) = &pairs[0].1 {
                if let CborValue::Map(inner_pairs) = &arr[0] {
                    let keys: Vec<&str> = inner_pairs
                        .iter()
                        .map(|(k, _)| k.as_text().unwrap())
                        .collect();
                    assert_eq!(keys, vec!["a", "zz"]);
                } else {
                    panic!("Expected map inside array");
                }
            } else {
                panic!("Expected array");
            }
        }
    }

    // Serialization and value conversion

    #[test]
    fn to_bytes_produces_valid_cbor() {
        let mut map = CborCanonicalMap::new();
        map.insert("key", CborValue::Text("value".to_string()));

        let bytes = map.to_bytes().expect("should serialize to bytes");
        assert!(!bytes.is_empty());

        // Deserialize back
        let deserialized: CborValue =
            ciborium::de::from_reader(&bytes[..]).expect("should deserialize");
        if let CborValue::Map(pairs) = deserialized {
            assert_eq!(pairs.len(), 1);
            assert_eq!(
                pairs[0],
                (
                    CborValue::Text("key".to_string()),
                    CborValue::Text("value".to_string())
                )
            );
        } else {
            panic!("Expected map after deserialization");
        }
    }

    #[test]
    fn to_bytes_sorts_before_serializing() {
        let mut map = CborCanonicalMap::new();
        map.insert("beta", CborValue::Integer(2.into()));
        map.insert("a", CborValue::Integer(1.into()));

        let bytes = map.to_bytes().expect("should serialize");
        let deserialized: CborValue =
            ciborium::de::from_reader(&bytes[..]).expect("should deserialize");

        if let CborValue::Map(pairs) = deserialized {
            let keys: Vec<&str> = pairs.iter().map(|(k, _)| k.as_text().unwrap()).collect();
            assert_eq!(keys, vec!["a", "beta"]);
        }
    }

    #[test]
    fn to_value_unsorted_preserves_insertion_order() {
        let mut map = CborCanonicalMap::new();
        map.insert("z", CborValue::Integer(1.into()));
        map.insert("a", CborValue::Integer(2.into()));

        let value = map.to_value_unsorted();
        if let CborValue::Map(pairs) = value {
            assert_eq!(pairs[0].0, CborValue::Text("z".to_string()));
            assert_eq!(pairs[1].0, CborValue::Text("a".to_string()));
        }
    }

    #[test]
    fn to_value_sorted_returns_sorted_map() {
        let mut map = CborCanonicalMap::new();
        map.insert("beta", CborValue::Integer(2.into()));
        map.insert("a", CborValue::Integer(1.into()));

        let value = map.to_value_sorted();
        if let CborValue::Map(pairs) = value {
            assert_eq!(pairs[0].0, CborValue::Text("a".to_string()));
            assert_eq!(pairs[1].0, CborValue::Text("beta".to_string()));
        }
    }

    #[test]
    fn to_value_clone_returns_sorted_clone() {
        let mut map = CborCanonicalMap::new();
        map.insert("beta", CborValue::Integer(2.into()));
        map.insert("a", CborValue::Integer(1.into()));

        let value = map.to_value_clone();
        if let CborValue::Map(pairs) = value {
            assert_eq!(pairs[0].0, CborValue::Text("a".to_string()));
            assert_eq!(pairs[1].0, CborValue::Text("beta".to_string()));
        }

        // Original map should still be accessible (it was sorted in place but not consumed)
        let val = ValuesCollection::get(&map, &CborValue::Text("a".to_string()));
        assert!(val.is_some());
    }

    // TryFrom and From impls

    #[test]
    fn try_from_cbor_map_succeeds() {
        let cbor = CborValue::Map(vec![(
            CborValue::Text("k".to_string()),
            CborValue::Bool(true),
        )]);

        let map = CborCanonicalMap::try_from(cbor).expect("should convert from map");
        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("k".to_string())),
            Some(&CborValue::Bool(true))
        );
    }

    #[test]
    fn try_from_non_map_fails() {
        let cbor = CborValue::Text("not a map".to_string());
        let result = CborCanonicalMap::try_from(cbor);
        assert!(result.is_err());
    }

    #[test]
    fn from_vec_creates_canonical_map() {
        let vec = vec![(
            CborValue::Text("x".to_string()),
            CborValue::Integer(42.into()),
        )];

        let map: CborCanonicalMap = vec.into();
        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("x".to_string())),
            Some(&CborValue::Integer(42.into()))
        );
    }

    #[test]
    fn from_ref_vec_creates_canonical_map() {
        let vec = vec![(
            CborValue::Text("y".to_string()),
            CborValue::Integer(7.into()),
        )];

        let map: CborCanonicalMap = (&vec).into();
        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("y".to_string())),
            Some(&CborValue::Integer(7.into()))
        );
    }

    #[test]
    fn from_btreemap_string_creates_canonical_map() {
        let mut btree = BTreeMap::new();
        btree.insert("alpha".to_string(), CborValue::Integer(1.into()));
        btree.insert("beta".to_string(), CborValue::Integer(2.into()));

        let map: CborCanonicalMap = (&btree).into();
        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("alpha".to_string())),
            Some(&CborValue::Integer(1.into()))
        );
        assert_eq!(
            ValuesCollection::get(&map, &CborValue::Text("beta".to_string())),
            Some(&CborValue::Integer(2.into()))
        );
    }

    // ValuesCollection trait impl

    #[test]
    fn values_collection_get_returns_value() {
        let map = CborCanonicalMap::from_vector(vec![(
            CborValue::Text("k".to_string()),
            CborValue::Text("v".to_string()),
        )]);

        let result = ValuesCollection::get(&map, &CborValue::Text("k".to_string()));
        assert_eq!(result, Some(&CborValue::Text("v".to_string())));
    }

    #[test]
    fn values_collection_get_returns_none_for_missing() {
        let map = CborCanonicalMap::new();
        let result = ValuesCollection::get(&map, &CborValue::Text("missing".to_string()));
        assert!(result.is_none());
    }

    #[test]
    fn values_collection_remove_returns_removed_value() {
        let mut map = CborCanonicalMap::from_vector(vec![
            (
                CborValue::Text("a".to_string()),
                CborValue::Integer(1.into()),
            ),
            (
                CborValue::Text("b".to_string()),
                CborValue::Integer(2.into()),
            ),
        ]);

        let removed = ValuesCollection::remove(&mut map, "a");
        assert_eq!(removed, Some(CborValue::Integer(1.into())));
        assert!(ValuesCollection::get(&map, &CborValue::Text("a".to_string())).is_none());
    }

    #[test]
    fn values_collection_remove_returns_none_for_missing() {
        let mut map = CborCanonicalMap::new();
        let removed = ValuesCollection::remove(&mut map, "nonexistent");
        assert!(removed.is_none());
    }

    // replace_paths (ReplacePaths trait)

    #[test]
    fn replace_paths_converts_multiple_paths() {
        let cbor_value = cbor!({
            "field1" => vec![0_u8; 32],
            "field2" => vec![1_u8; 32]
        })
        .expect("valid cbor");

        let mut canonical: CborCanonicalMap = cbor_value.try_into().expect("valid canonical");
        ReplacePaths::replace_paths(
            &mut canonical,
            vec!["field1", "field2"],
            FieldType::ArrayInt,
            FieldType::Bytes,
        );

        let v1 = ValuesCollection::get(&canonical, &CborValue::Text("field1".to_string()));
        assert!(matches!(v1, Some(CborValue::Bytes(_))));
        let v2 = ValuesCollection::get(&canonical, &CborValue::Text("field2".to_string()));
        assert!(matches!(v2, Some(CborValue::Bytes(_))));
    }

    #[test]
    fn replace_path_returns_none_for_nonexistent_path() {
        let mut map = CborCanonicalMap::new();
        map.insert("exists", CborValue::Text("value".to_string()));

        let result =
            ReplacePaths::replace_path(&mut map, "nonexistent", FieldType::Bytes, FieldType::Bytes);
        assert!(result.is_none());
    }

    #[test]
    fn get_path_mut_with_empty_path_returns_none() {
        let mut map = CborCanonicalMap::new();
        map.insert("key", CborValue::Integer(1.into()));

        // Empty string results in a path with a single empty-string key
        // which won't match any real keys typically
        let result = ReplacePaths::get_path_mut(&mut map, "");
        // An empty path string still produces a single Key("") step,
        // which won't match any inserted key
        assert!(result.is_none());
    }

    // value_to_bytes tests

    #[test]
    fn value_to_bytes_from_bytes() {
        let val = CborValue::Bytes(vec![1, 2, 3, 4]);
        let result = value_to_bytes(&val).expect("should succeed");
        assert_eq!(result, Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn value_to_bytes_from_valid_base58_text() {
        let original = vec![1, 2, 3, 4, 5];
        let encoded = bs58::encode(&original).into_string();
        let val = CborValue::Text(encoded);

        let result = value_to_bytes(&val).expect("should succeed");
        assert_eq!(result, Some(original));
    }

    #[test]
    fn value_to_bytes_from_invalid_base58_text_returns_none() {
        // "0OIl" contains characters invalid in base58
        let val = CborValue::Text("0OIl!!!".to_string());
        let result = value_to_bytes(&val).expect("should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn value_to_bytes_from_integer_array() {
        let val = CborValue::Array(vec![
            CborValue::Integer(10.into()),
            CborValue::Integer(20.into()),
            CborValue::Integer(30.into()),
        ]);

        let result = value_to_bytes(&val).expect("should succeed");
        assert_eq!(result, Some(vec![10, 20, 30]));
    }

    #[test]
    fn value_to_bytes_from_array_with_non_integer_fails() {
        let val = CborValue::Array(vec![
            CborValue::Integer(1.into()),
            CborValue::Text("not an int".to_string()),
        ]);

        let result = value_to_bytes(&val);
        assert!(result.is_err());
    }

    #[test]
    fn value_to_bytes_from_bool_fails() {
        let val = CborValue::Bool(true);
        let result = value_to_bytes(&val);
        assert!(result.is_err());
    }

    #[test]
    fn value_to_bytes_from_null_fails() {
        let val = CborValue::Null;
        let result = value_to_bytes(&val);
        assert!(result.is_err());
    }

    // value_to_hash tests

    #[test]
    fn value_to_hash_from_32_bytes() {
        let bytes = vec![42u8; 32];
        let val = CborValue::Bytes(bytes.clone());

        let result = value_to_hash(&val).expect("should succeed");
        assert_eq!(result, [42u8; 32]);
    }

    #[test]
    fn value_to_hash_from_wrong_length_bytes_fails() {
        let val = CborValue::Bytes(vec![1u8; 16]);
        let result = value_to_hash(&val);
        assert!(result.is_err());
    }

    #[test]
    fn value_to_hash_from_valid_base58_text_32_bytes() {
        let original = [7u8; 32];
        let encoded = bs58::encode(&original).into_string();
        let val = CborValue::Text(encoded);

        let result = value_to_hash(&val).expect("should succeed");
        assert_eq!(result, original);
    }

    #[test]
    fn value_to_hash_from_invalid_base58_text_fails() {
        let val = CborValue::Text("!!!invalid!!!".to_string());
        let result = value_to_hash(&val);
        assert!(result.is_err());
    }

    #[test]
    fn value_to_hash_from_base58_text_wrong_length_fails() {
        // Valid base58 but only 4 bytes
        let encoded = bs58::encode(&[1u8; 4]).into_string();
        let val = CborValue::Text(encoded);
        let result = value_to_hash(&val);
        assert!(result.is_err());
    }

    #[test]
    fn value_to_hash_from_integer_array_32_bytes() {
        let val = CborValue::Array((0..32).map(|i| CborValue::Integer(i.into())).collect());

        let result = value_to_hash(&val).expect("should succeed");
        let expected: [u8; 32] = (0u8..32).collect::<Vec<u8>>().try_into().unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn value_to_hash_from_integer_array_wrong_length_fails() {
        let val = CborValue::Array(vec![CborValue::Integer(1.into()); 10]);
        let result = value_to_hash(&val);
        assert!(result.is_err());
    }

    #[test]
    fn value_to_hash_from_array_with_non_integer_fails() {
        let mut arr: Vec<CborValue> = (0..31).map(|i| CborValue::Integer(i.into())).collect();
        arr.push(CborValue::Text("not_int".to_string()));
        let val = CborValue::Array(arr);

        let result = value_to_hash(&val);
        assert!(result.is_err());
    }

    #[test]
    fn value_to_hash_from_bool_fails() {
        let val = CborValue::Bool(false);
        let result = value_to_hash(&val);
        assert!(result.is_err());
    }

    // Round-trip: to_bytes and back

    #[test]
    fn round_trip_canonical_map_through_bytes() {
        let mut map = CborCanonicalMap::new();
        map.insert("name", CborValue::Text("Alice".to_string()));
        map.insert("age", CborValue::Integer(30.into()));
        map.insert("active", CborValue::Bool(true));

        let bytes = map.to_bytes().expect("should serialize");

        let decoded: CborValue = ciborium::de::from_reader(&bytes[..]).expect("should deserialize");
        let decoded_map = CborCanonicalMap::try_from(decoded).expect("should convert to map");

        assert_eq!(
            ValuesCollection::get(&decoded_map, &CborValue::Text("name".to_string())),
            Some(&CborValue::Text("Alice".to_string()))
        );
        assert_eq!(
            ValuesCollection::get(&decoded_map, &CborValue::Text("age".to_string())),
            Some(&CborValue::Integer(30.into()))
        );
        assert_eq!(
            ValuesCollection::get(&decoded_map, &CborValue::Text("active".to_string())),
            Some(&CborValue::Bool(true))
        );
    }

    #[test]
    fn canonical_sort_with_same_length_keys_uses_lexicographic_order() {
        let mut map = CborCanonicalMap::new();
        map.insert("cc", CborValue::Integer(1.into()));
        map.insert("bb", CborValue::Integer(2.into()));
        map.insert("aa", CborValue::Integer(3.into()));

        map.sort_canonical();

        let value = map.to_value_unsorted();
        if let CborValue::Map(pairs) = value {
            let keys: Vec<&str> = pairs.iter().map(|(k, _)| k.as_text().unwrap()).collect();
            assert_eq!(keys, vec!["aa", "bb", "cc"]);
        }
    }

    #[test]
    fn canonical_sort_shorter_keys_come_first() {
        let mut map = CborCanonicalMap::new();
        map.insert("zzz", CborValue::Integer(1.into()));
        map.insert("a", CborValue::Integer(2.into()));
        map.insert("bb", CborValue::Integer(3.into()));

        map.sort_canonical();

        let value = map.to_value_unsorted();
        if let CborValue::Map(pairs) = value {
            let keys: Vec<&str> = pairs.iter().map(|(k, _)| k.as_text().unwrap()).collect();
            assert_eq!(keys, vec!["a", "bb", "zzz"]);
        }
    }
}
