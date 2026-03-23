use ciborium::value::Value as CborValue;
use std::borrow::Borrow;
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::{collections::BTreeMap, convert::TryInto};

use crate::util::cbor_value::value_to_hash;
use crate::ProtocolError;

pub trait CborBTreeMapHelper {
    fn get_optional_identifier(&self, key: &str) -> Result<Option<[u8; 32]>, ProtocolError>;
    fn get_identifier(&self, key: &str) -> Result<[u8; 32], ProtocolError>;
    fn get_optional_string(&self, key: &str) -> Result<Option<String>, ProtocolError>;
    fn get_string(&self, key: &str) -> Result<String, ProtocolError>;
    fn get_optional_str(&self, key: &str) -> Result<Option<&str>, ProtocolError>;
    fn get_str(&self, key: &str) -> Result<&str, ProtocolError>;
    fn get_optional_integer<T: TryFrom<i128>>(&self, key: &str)
        -> Result<Option<T>, ProtocolError>;
    fn get_integer<T: TryFrom<i128>>(&self, key: &str) -> Result<T, ProtocolError>;
    fn get_optional_bool(&self, key: &str) -> Result<Option<bool>, ProtocolError>;
    fn get_bool(&self, key: &str) -> Result<bool, ProtocolError>;
    fn get_optional_inner_value_array<'a, I: FromIterator<&'a CborValue>>(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, ProtocolError>;
    fn get_inner_value_array<'a, I: FromIterator<&'a CborValue>>(
        &'a self,
        key: &str,
    ) -> Result<I, ProtocolError>;
    fn get_optional_inner_string_array<I: FromIterator<String>>(
        &self,
        key: &str,
    ) -> Result<Option<I>, ProtocolError>;
    fn get_inner_string_array<I: FromIterator<String>>(
        &self,
        key: &str,
    ) -> Result<I, ProtocolError>;
    fn get_optional_inner_borrowed_str_value_map<'a, I: FromIterator<(String, &'a CborValue)>>(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, ProtocolError>;
    fn get_optional_inner_borrowed_map(
        &self,
        key: &str,
    ) -> Result<Option<&Vec<(CborValue, CborValue)>>, ProtocolError>;
    fn get_inner_borrowed_str_value_map<'a, I: FromIterator<(String, &'a CborValue)>>(
        &'a self,
        key: &str,
    ) -> Result<I, ProtocolError>;

    fn remove_optional_integer<T: TryFrom<i128>>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, ProtocolError>;
    fn remove_integer<T: TryFrom<i128>>(&mut self, key: &str) -> Result<T, ProtocolError>;
}

pub trait CborMapExtension {
    fn as_u16(&self, key: &str, error_message: &str) -> Result<u16, ProtocolError>;
    fn as_u8(&self, key: &str, error_message: &str) -> Result<u8, ProtocolError>;
    fn as_bool(&self, key: &str, error_message: &str) -> Result<bool, ProtocolError>;
    fn as_bytes(&self, key: &str, error_message: &str) -> Result<Vec<u8>, ProtocolError>;
    fn as_string(&self, key: &str, error_message: &str) -> Result<String, ProtocolError>;
    fn as_u64(&self, key: &str, error_message: &str) -> Result<u64, ProtocolError>;
}

impl<V> CborBTreeMapHelper for BTreeMap<String, V>
where
    V: Borrow<CborValue>,
{
    fn get_optional_identifier(&self, key: &str) -> Result<Option<[u8; 32]>, ProtocolError> {
        self.get(key).map(|i| value_to_hash(i.borrow())).transpose()
    }

    fn get_identifier(&self, key: &str) -> Result<[u8; 32], ProtocolError> {
        self.get_optional_identifier(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get identifier property {key}"))
        })
    }

    fn get_optional_string(&self, key: &str) -> Result<Option<String>, ProtocolError> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_text()
                    .map(|str| str.to_string())
                    .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a string")))
            })
            .transpose()
    }

    fn get_string(&self, key: &str) -> Result<String, ProtocolError> {
        self.get_optional_string(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get string property {key}"))
        })
    }

    fn get_optional_str(&self, key: &str) -> Result<Option<&str>, ProtocolError> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_text()
                    .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a string")))
            })
            .transpose()
    }

    fn get_str(&self, key: &str) -> Result<&str, ProtocolError> {
        self.get_optional_str(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get str property {key}"))
        })
    }

    fn get_optional_integer<T: TryFrom<i128>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, ProtocolError> {
        self.get(key)
            .map(|v| {
                if v.borrow().is_null() {
                    Ok::<Option<Result<T, ProtocolError>>, ProtocolError>(None)
                } else {
                    Ok(Some(
                        i128::from(v.borrow().as_integer().ok_or_else(|| {
                            ProtocolError::DecodingError(format!("{key} must be an integer"))
                        })?)
                        .try_into()
                        .map_err(|_| {
                            ProtocolError::DecodingError(format!("{key} is out of required bounds"))
                        }),
                    ))
                }
            })
            .transpose()?
            .flatten()
            .transpose()
    }

    fn get_integer<T: TryFrom<i128>>(&self, key: &str) -> Result<T, ProtocolError> {
        self.get_optional_integer(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get integer property {key}"))
        })
    }

    fn get_optional_bool(&self, key: &str) -> Result<Option<bool>, ProtocolError> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_bool()
                    .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a bool")))
            })
            .transpose()
    }

    fn get_bool(&self, key: &str) -> Result<bool, ProtocolError> {
        self.get_optional_bool(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get bool property {key}"))
        })
    }

    fn remove_optional_integer<T: TryFrom<i128>>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, ProtocolError> {
        self.remove(key)
            .map(|v| {
                if v.borrow().is_null() {
                    Ok::<Option<Result<T, ProtocolError>>, ProtocolError>(None)
                } else {
                    Ok(Some(
                        i128::from(v.borrow().as_integer().ok_or_else(|| {
                            ProtocolError::DecodingError(format!("{key} must be an integer"))
                        })?)
                        .try_into()
                        .map_err(|_| {
                            ProtocolError::DecodingError(format!("{key} is out of required bounds"))
                        }),
                    ))
                }
            })
            .transpose()?
            .flatten()
            .transpose()
    }

    fn remove_integer<T: TryFrom<i128>>(&mut self, key: &str) -> Result<T, ProtocolError> {
        self.remove_optional_integer(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to remove integer property {key}"))
        })
    }

    fn get_optional_inner_value_array<'a, I: FromIterator<&'a CborValue>>(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, ProtocolError> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_array()
                    .map(|vec| vec.iter().collect())
                    .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a bool")))
            })
            .transpose()
    }

    fn get_inner_value_array<'a, I: FromIterator<&'a CborValue>>(
        &'a self,
        key: &str,
    ) -> Result<I, ProtocolError> {
        self.get_optional_inner_value_array(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get inner value array property {key}"))
        })
    }

    fn get_optional_inner_string_array<I: FromIterator<String>>(
        &self,
        key: &str,
    ) -> Result<Option<I>, ProtocolError> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_array()
                    .map(|inner| {
                        inner
                            .iter()
                            .map(|v| {
                                let Some(str) = v.as_text() else {
                                    return Err(ProtocolError::DecodingError(format!(
                                        "{key} must be an string"
                                    )));
                                };
                                Ok(str.to_string())
                            })
                            .collect::<Result<I, ProtocolError>>()
                    })
                    .transpose()?
                    .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a bool")))
            })
            .transpose()
    }

    fn get_inner_string_array<I: FromIterator<String>>(
        &self,
        key: &str,
    ) -> Result<I, ProtocolError> {
        self.get_optional_inner_string_array(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get inner string property {key}"))
        })
    }

    fn get_optional_inner_borrowed_str_value_map<
        'a,
        I: FromIterator<(String, &'a ciborium::Value)>,
    >(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, ProtocolError> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_map()
                    .map(|inner| {
                        inner
                            .iter()
                            .map(|(k, v)| {
                                let Some(str) = k.as_text() else {
                                    return Err(ProtocolError::DecodingError(format!(
                                        "{key} must be an string"
                                    )));
                                };
                                Ok((str.to_string(), v))
                            })
                            .collect::<Result<I, ProtocolError>>()
                    })
                    .transpose()?
                    .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a bool")))
            })
            .transpose()
    }

    fn get_optional_inner_borrowed_map(
        &self,
        key: &str,
    ) -> Result<Option<&Vec<(CborValue, CborValue)>>, ProtocolError> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_map()
                    .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a map")))
            })
            .transpose()
    }

    fn get_inner_borrowed_str_value_map<'a, I: FromIterator<(String, &'a ciborium::Value)>>(
        &'a self,
        key: &str,
    ) -> Result<I, ProtocolError> {
        self.get_optional_inner_borrowed_str_value_map(key)?
            .ok_or_else(|| {
                ProtocolError::DecodingError(format!(
                    "unable to get borrowed str value map property {key}"
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::value::Value as CborValue;
    use std::collections::BTreeMap;

    fn make_map(pairs: Vec<(&str, CborValue)>) -> BTreeMap<String, CborValue> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    // --- get_optional_string / get_string ---

    #[test]
    fn get_optional_string_present() {
        let map = make_map(vec![("name", CborValue::Text("Alice".to_string()))]);
        let result = map.get_optional_string("name").unwrap();
        assert_eq!(result, Some("Alice".to_string()));
    }

    #[test]
    fn get_optional_string_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result = map.get_optional_string("name").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_string_wrong_type() {
        let map = make_map(vec![("name", CborValue::Integer(42.into()))]);
        let result = map.get_optional_string("name");
        assert!(result.is_err());
    }

    #[test]
    fn get_string_present() {
        let map = make_map(vec![("name", CborValue::Text("Bob".to_string()))]);
        let result = map.get_string("name").unwrap();
        assert_eq!(result, "Bob");
    }

    #[test]
    fn get_string_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result = map.get_string("name");
        assert!(result.is_err());
    }

    // --- get_optional_str / get_str ---

    #[test]
    fn get_optional_str_present() {
        let map = make_map(vec![("key", CborValue::Text("value".to_string()))]);
        let result = map.get_optional_str("key").unwrap();
        assert_eq!(result, Some("value"));
    }

    #[test]
    fn get_optional_str_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result = map.get_optional_str("key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_str_present() {
        let map = make_map(vec![("key", CborValue::Text("val".to_string()))]);
        let result = map.get_str("key").unwrap();
        assert_eq!(result, "val");
    }

    #[test]
    fn get_str_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result = map.get_str("key");
        assert!(result.is_err());
    }

    // --- get_optional_integer / get_integer ---

    #[test]
    fn get_optional_integer_present() {
        let map = make_map(vec![("count", CborValue::Integer(42.into()))]);
        let result: Option<i64> = map.get_optional_integer("count").unwrap();
        assert_eq!(result, Some(42));
    }

    #[test]
    fn get_optional_integer_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result: Option<i64> = map.get_optional_integer("count").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_integer_null_value() {
        let map = make_map(vec![("count", CborValue::Null)]);
        let result: Option<i64> = map.get_optional_integer("count").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_integer_wrong_type() {
        let map = make_map(vec![("count", CborValue::Text("not_int".to_string()))]);
        let result: Result<Option<i64>, _> = map.get_optional_integer("count");
        assert!(result.is_err());
    }

    #[test]
    fn get_integer_present() {
        let map = make_map(vec![("count", CborValue::Integer(100.into()))]);
        let result: i64 = map.get_integer("count").unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn get_integer_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result: Result<i64, _> = map.get_integer("count");
        assert!(result.is_err());
    }

    // --- get_optional_bool / get_bool ---

    #[test]
    fn get_optional_bool_present() {
        let map = make_map(vec![("flag", CborValue::Bool(true))]);
        let result = map.get_optional_bool("flag").unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn get_optional_bool_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result = map.get_optional_bool("flag").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_bool_wrong_type() {
        let map = make_map(vec![("flag", CborValue::Integer(1.into()))]);
        let result = map.get_optional_bool("flag");
        assert!(result.is_err());
    }

    #[test]
    fn get_bool_present() {
        let map = make_map(vec![("flag", CborValue::Bool(false))]);
        let result = map.get_bool("flag").unwrap();
        assert!(!result);
    }

    #[test]
    fn get_bool_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result = map.get_bool("flag");
        assert!(result.is_err());
    }

    // --- get_optional_identifier / get_identifier ---

    #[test]
    fn get_optional_identifier_with_bytes() {
        let id_bytes = [7u8; 32];
        let map = make_map(vec![("id", CborValue::Bytes(id_bytes.to_vec()))]);
        let result = map.get_optional_identifier("id").unwrap();
        assert_eq!(result, Some(id_bytes));
    }

    #[test]
    fn get_optional_identifier_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result = map.get_optional_identifier("id").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_identifier_present() {
        let id_bytes = [3u8; 32];
        let map = make_map(vec![("id", CborValue::Bytes(id_bytes.to_vec()))]);
        let result = map.get_identifier("id").unwrap();
        assert_eq!(result, id_bytes);
    }

    #[test]
    fn get_identifier_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result = map.get_identifier("id");
        assert!(result.is_err());
    }

    // --- remove_optional_integer / remove_integer ---

    #[test]
    fn remove_optional_integer_present() {
        let mut map = make_map(vec![("val", CborValue::Integer(99.into()))]);
        let result: Option<i64> = map.remove_optional_integer("val").unwrap();
        assert_eq!(result, Some(99));
        assert!(!map.contains_key("val"));
    }

    #[test]
    fn remove_optional_integer_absent() {
        let mut map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result: Option<i64> = map.remove_optional_integer("val").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn remove_optional_integer_null() {
        let mut map = make_map(vec![("val", CborValue::Null)]);
        let result: Option<i64> = map.remove_optional_integer("val").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn remove_integer_present() {
        let mut map = make_map(vec![("val", CborValue::Integer(50.into()))]);
        let result: i64 = map.remove_integer("val").unwrap();
        assert_eq!(result, 50);
    }

    #[test]
    fn remove_integer_absent() {
        let mut map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result: Result<i64, _> = map.remove_integer("val");
        assert!(result.is_err());
    }

    // --- get_optional_inner_value_array / get_inner_value_array ---

    #[test]
    fn get_optional_inner_value_array_present() {
        let array = vec![CborValue::Integer(1.into()), CborValue::Integer(2.into())];
        let map = make_map(vec![("arr", CborValue::Array(array))]);
        let result: Option<Vec<&CborValue>> = map.get_optional_inner_value_array("arr").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn get_optional_inner_value_array_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result: Option<Vec<&CborValue>> = map.get_optional_inner_value_array("arr").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_inner_value_array_present() {
        let array = vec![CborValue::Bool(true)];
        let map = make_map(vec![("arr", CborValue::Array(array))]);
        let result: Vec<&CborValue> = map.get_inner_value_array("arr").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn get_inner_value_array_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result: Result<Vec<&CborValue>, _> = map.get_inner_value_array("arr");
        assert!(result.is_err());
    }

    // --- get_optional_inner_string_array / get_inner_string_array ---

    #[test]
    fn get_optional_inner_string_array_present() {
        let array = vec![
            CborValue::Text("hello".to_string()),
            CborValue::Text("world".to_string()),
        ];
        let map = make_map(vec![("strs", CborValue::Array(array))]);
        let result: Option<Vec<String>> = map.get_optional_inner_string_array("strs").unwrap();
        assert_eq!(result, Some(vec!["hello".to_string(), "world".to_string()]));
    }

    #[test]
    fn get_optional_inner_string_array_with_non_string_element() {
        let array = vec![
            CborValue::Text("hello".to_string()),
            CborValue::Integer(42.into()),
        ];
        let map = make_map(vec![("strs", CborValue::Array(array))]);
        let result: Result<Option<Vec<String>>, _> = map.get_optional_inner_string_array("strs");
        assert!(result.is_err());
    }

    #[test]
    fn get_inner_string_array_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result: Result<Vec<String>, _> = map.get_inner_string_array("strs");
        assert!(result.is_err());
    }

    // --- get_optional_inner_borrowed_map ---

    #[test]
    fn get_optional_inner_borrowed_map_present() {
        let inner_map = vec![(
            CborValue::Text("k".to_string()),
            CborValue::Integer(1.into()),
        )];
        let map = make_map(vec![("m", CborValue::Map(inner_map))]);
        let result = map.get_optional_inner_borrowed_map("m").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn get_optional_inner_borrowed_map_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result = map.get_optional_inner_borrowed_map("m").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_optional_inner_borrowed_map_wrong_type() {
        let map = make_map(vec![("m", CborValue::Integer(1.into()))]);
        let result = map.get_optional_inner_borrowed_map("m");
        assert!(result.is_err());
    }

    // --- get_optional_inner_borrowed_str_value_map / get_inner_borrowed_str_value_map ---

    #[test]
    fn get_optional_inner_borrowed_str_value_map_present() {
        let inner_map = vec![(
            CborValue::Text("key".to_string()),
            CborValue::Integer(42.into()),
        )];
        let map = make_map(vec![("m", CborValue::Map(inner_map))]);
        let result: Option<BTreeMap<String, &CborValue>> =
            map.get_optional_inner_borrowed_str_value_map("m").unwrap();
        assert!(result.is_some());
        let inner = result.unwrap();
        assert_eq!(inner.len(), 1);
        assert!(inner.contains_key("key"));
    }

    #[test]
    fn get_inner_borrowed_str_value_map_absent() {
        let map: BTreeMap<String, CborValue> = BTreeMap::new();
        let result: Result<BTreeMap<String, &CborValue>, _> =
            map.get_inner_borrowed_str_value_map("m");
        assert!(result.is_err());
    }
}
