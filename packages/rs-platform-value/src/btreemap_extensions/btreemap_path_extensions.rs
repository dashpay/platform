#[cfg(feature = "json")]
use serde_json::Value as JsonValue;
use std::borrow::Borrow;
use std::convert::TryFrom;
use std::iter::FromIterator;

use std::collections::BTreeMap;
#[cfg(feature = "json")]
use std::convert::TryInto;

use crate::value_map::ValueMapHelper;
use crate::{Error, Identifier, Value};

pub trait BTreeValueMapPathHelper {
    fn get_at_path(&self, path: &str) -> Result<&Value, Error>;
    fn get_optional_at_path(&self, path: &str) -> Result<Option<&Value>, Error>;
    fn get_optional_identifier_at_path(&self, path: &str) -> Result<Option<[u8; 32]>, Error>;
    fn get_identifier_at_path(&self, path: &str) -> Result<[u8; 32], Error>;
    fn get_optional_string_at_path(&self, path: &str) -> Result<Option<String>, Error>;
    fn get_string_at_path(&self, path: &str) -> Result<String, Error>;
    fn get_optional_str_at_path(&self, path: &str) -> Result<Option<&str>, Error>;
    fn get_str_at_path(&self, path: &str) -> Result<&str, Error>;
    fn get_optional_float_at_path(&self, path: &str) -> Result<Option<f64>, Error>;
    fn get_float_at_path(&self, path: &str) -> Result<f64, Error>;
    fn get_optional_integer_at_path<T>(&self, path: &str) -> Result<Option<T>, Error>
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
            + TryFrom<i8>;
    fn get_integer_at_path<T>(&self, path: &str) -> Result<T, Error>
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
            + TryFrom<i8>;
    fn get_optional_bool_at_path(&self, path: &str) -> Result<Option<bool>, Error>;
    fn get_bool_at_path(&self, path: &str) -> Result<bool, Error>;
    fn get_optional_inner_value_array_at_path<'a, I: FromIterator<&'a Value>>(
        &'a self,
        path: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_value_array_at_path<'a, I: FromIterator<&'a Value>>(
        &'a self,
        path: &str,
    ) -> Result<I, Error>;
    fn get_optional_inner_string_array_at_path<I: FromIterator<String>>(
        &self,
        path: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_string_array_at_path<I: FromIterator<String>>(
        &self,
        path: &str,
    ) -> Result<I, Error>;
    fn get_optional_inner_borrowed_map_at_path(
        &self,
        path: &str,
    ) -> Result<Option<&Vec<(Value, Value)>>, Error>;
    fn get_optional_inner_borrowed_str_value_map_at_path<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        path: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_borrowed_str_value_map_at_path<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        path: &str,
    ) -> Result<I, Error>;
    #[cfg(feature = "json")]
    fn get_optional_inner_str_json_value_map_at_path<I: FromIterator<(String, JsonValue)>>(
        &self,
        path: &str,
    ) -> Result<Option<I>, Error>;
    #[cfg(feature = "json")]
    fn get_inner_str_json_value_map_at_path<I: FromIterator<(String, JsonValue)>>(
        &self,
        path: &str,
    ) -> Result<I, Error>;
    fn get_optional_hash256_bytes_at_path(&self, path: &str) -> Result<Option<[u8; 32]>, Error>;
    fn get_hash256_bytes_at_path(&self, path: &str) -> Result<[u8; 32], Error>;
    fn get_optional_identifier_bytes_at_path(&self, path: &str) -> Result<Option<Vec<u8>>, Error>;
    fn get_identifier_bytes_at_path(&self, path: &str) -> Result<Vec<u8>, Error>;
    fn remove_optional_string_at_path(&mut self, path: &str) -> Result<Option<String>, Error>;
    fn remove_string_at_path(&mut self, path: &str) -> Result<String, Error>;
    fn remove_optional_float_at_path(&mut self, path: &str) -> Result<Option<f64>, Error>;
    fn remove_float_at_path(&mut self, path: &str) -> Result<f64, Error>;
    fn remove_optional_integer_at_path<T>(&mut self, path: &str) -> Result<Option<T>, Error>
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
            + TryFrom<i8>;
    fn remove_integer_at_path<T>(&mut self, path: &str) -> Result<T, Error>
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
            + TryFrom<i8>;
    fn remove_optional_hash256_bytes_at_path(
        &mut self,
        path: &str,
    ) -> Result<Option<[u8; 32]>, Error>;
    fn remove_hash256_bytes_at_path(&mut self, path: &str) -> Result<[u8; 32], Error>;
    fn remove_optional_identifier_at_path(
        &mut self,
        path: &str,
    ) -> Result<Option<Identifier>, Error>;
    fn remove_identifier_at_path(&mut self, path: &str) -> Result<Identifier, Error>;
    fn get_optional_bytes_at_path(&self, path: &str) -> Result<Option<Vec<u8>>, Error>;
    fn get_bytes_at_path(&self, path: &str) -> Result<Vec<u8>, Error>;
    fn get_optional_binary_bytes_at_path(&self, path: &str) -> Result<Option<Vec<u8>>, Error>;
    fn get_binary_bytes_at_path(&self, path: &str) -> Result<Vec<u8>, Error>;
}

impl<V> BTreeValueMapPathHelper for BTreeMap<String, V>
where
    V: Borrow<Value>,
{
    fn get_at_path(&self, path: &str) -> Result<&Value, Error> {
        let mut split = path.split('.');
        let first = split.next();
        let Some(first_path_component) = first else {
            return Err(Error::PathError("path was empty".to_string()));
        };
        let mut current_value = self
            .get(first_path_component)
            .ok_or_else(|| {
                Error::StructureError(format!(
                    "unable to get property {first_path_component} in {path}"
                ))
            })?
            .borrow();
        for path_component in split {
            let map = current_value.to_map_ref()?;
            current_value = map.get_optional_key(path_component).ok_or_else(|| {
                Error::StructureError(format!(
                    "unable to get property at path {path_component} in {path}"
                ))
            })?;
        }
        Ok(current_value)
    }

    fn get_optional_at_path(&self, path: &str) -> Result<Option<&Value>, Error> {
        let mut split = path.split('.');
        let first = split.next();
        let Some(first_path_component) = first else {
            return Err(Error::PathError("path was empty".to_string()));
        };
        let Some(mut current_value) = self.get(first_path_component).map(|v| v.borrow()) else {
            return Ok(None);
        };
        for path_component in split {
            let map = current_value.to_map_ref()?;
            let Some(new_value) = map.get_optional_key(path_component) else {
                return Ok(None);
            };
            current_value = new_value;
        }
        Ok(Some(current_value))
    }

    fn get_optional_identifier_at_path(&self, path: &str) -> Result<Option<[u8; 32]>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| v.to_hash256())
            .transpose()
    }

    fn get_identifier_at_path(&self, path: &str) -> Result<[u8; 32], Error> {
        self.get_optional_identifier_at_path(path)?.ok_or_else(|| {
            Error::StructureError(format!("unable to get identifier property {path}"))
        })
    }

    fn get_optional_string_at_path(&self, path: &str) -> Result<Option<String>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| {
                v.as_text()
                    .map(|str| str.to_string())
                    .ok_or_else(|| Error::StructureError(format!("{path} must be a string")))
            })
            .transpose()
    }

    fn get_string_at_path(&self, path: &str) -> Result<String, Error> {
        self.get_optional_string_at_path(path)?
            .ok_or_else(|| Error::StructureError(format!("unable to get string property {path}")))
    }

    fn get_optional_str_at_path(&self, path: &str) -> Result<Option<&str>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| {
                v.as_text()
                    .ok_or_else(|| Error::StructureError(format!("{path} must be a string")))
            })
            .transpose()
    }

    fn get_str_at_path(&self, path: &str) -> Result<&str, Error> {
        self.get_optional_str_at_path(path)?
            .ok_or_else(|| Error::StructureError(format!("unable to get str property {path}")))
    }

    fn get_optional_integer_at_path<T>(&self, path: &str) -> Result<Option<T>, Error>
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
        self.get_optional_at_path(path)?
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_integer())
                }
            })
            .transpose()
    }

    fn get_integer_at_path<T>(&self, path: &str) -> Result<T, Error>
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
        self.get_optional_integer_at_path(path)?
            .ok_or_else(|| Error::StructureError(format!("unable to get integer property {path}")))
    }

    fn remove_optional_integer_at_path<T>(&mut self, path: &str) -> Result<Option<T>, Error>
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
        self.remove(path)
            .and_then(|v| {
                let borrowed = v.borrow();
                if borrowed.is_null() {
                    None
                } else {
                    Some(v.borrow().to_integer())
                }
            })
            .transpose()
    }

    fn remove_integer_at_path<T>(&mut self, path: &str) -> Result<T, Error>
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
        self.remove_optional_integer_at_path(path)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove integer property {path}"))
        })
    }

    fn get_optional_bool_at_path(&self, path: &str) -> Result<Option<bool>, Error> {
        self.get_optional_at_path(path)?
            .and_then(|v| if v.is_null() { None } else { Some(v.to_bool()) })
            .transpose()
    }

    fn get_bool_at_path(&self, path: &str) -> Result<bool, Error> {
        self.get_optional_bool_at_path(path)?
            .ok_or_else(|| Error::StructureError(format!("unable to get bool property {path}")))
    }

    fn get_optional_inner_value_array_at_path<'a, I: FromIterator<&'a Value>>(
        &'a self,
        path: &str,
    ) -> Result<Option<I>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| {
                v.as_array()
                    .map(|vec| vec.iter().collect())
                    .ok_or_else(|| Error::StructureError(format!("{path} must be a bool")))
            })
            .transpose()
    }

    fn get_inner_value_array_at_path<'a, I: FromIterator<&'a Value>>(
        &'a self,
        path: &str,
    ) -> Result<I, Error> {
        self.get_optional_inner_value_array_at_path(path)?
            .ok_or_else(|| {
                Error::StructureError(format!("unable to get inner value array property {path}"))
            })
    }

    fn get_optional_inner_string_array_at_path<I: FromIterator<String>>(
        &self,
        path: &str,
    ) -> Result<Option<I>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| {
                v.as_array()
                    .map(|inner| {
                        inner
                            .iter()
                            .map(|v| {
                                let Some(str) = v.as_text() else {
                                    return Err(Error::StructureError(format!(
                                        "{path} must be an string"
                                    )));
                                };
                                Ok(str.to_string())
                            })
                            .collect::<Result<I, Error>>()
                    })
                    .transpose()?
                    .ok_or_else(|| Error::StructureError(format!("{path} must be a bool")))
            })
            .transpose()
    }

    fn get_inner_string_array_at_path<I: FromIterator<String>>(
        &self,
        path: &str,
    ) -> Result<I, Error> {
        self.get_optional_inner_string_array_at_path(path)?
            .ok_or_else(|| {
                Error::StructureError(format!("unable to get inner string property {path}"))
            })
    }

    fn get_optional_inner_borrowed_map_at_path(
        &self,
        path: &str,
    ) -> Result<Option<&Vec<(Value, Value)>>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| {
                v.as_map()
                    .ok_or_else(|| Error::StructureError(format!("{path} must be a map")))
            })
            .transpose()
    }

    fn get_optional_inner_borrowed_str_value_map_at_path<
        'a,
        I: FromIterator<(String, &'a Value)>,
    >(
        &'a self,
        path: &str,
    ) -> Result<Option<I>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| {
                v.as_map()
                    .map(|inner| {
                        inner
                            .iter()
                            .map(|(k, v)| Ok((k.to_text()?, v)))
                            .collect::<Result<I, Error>>()
                    })
                    .transpose()?
                    .ok_or_else(|| Error::StructureError(format!("{path} must be a bool")))
            })
            .transpose()
    }

    fn get_inner_borrowed_str_value_map_at_path<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        path: &str,
    ) -> Result<I, Error> {
        self.get_optional_inner_borrowed_str_value_map_at_path(path)?
            .ok_or_else(|| {
                Error::StructureError(format!(
                    "unable to get borrowed str value map property {path}"
                ))
            })
    }

    #[cfg(feature = "json")]
    fn get_optional_inner_str_json_value_map_at_path<I: FromIterator<(String, JsonValue)>>(
        &self,
        path: &str,
    ) -> Result<Option<I>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| {
                v.as_map()
                    .map(|inner| {
                        inner
                            .iter()
                            .map(|(k, v)| Ok((k.to_text()?, v.clone().try_into()?)))
                            .collect::<Result<I, Error>>()
                    })
                    .transpose()?
                    .ok_or_else(|| Error::StructureError(format!("{path} must be a bool")))
            })
            .transpose()
    }

    #[cfg(feature = "json")]
    fn get_inner_str_json_value_map_at_path<I: FromIterator<(String, JsonValue)>>(
        &self,
        path: &str,
    ) -> Result<I, Error> {
        self.get_optional_inner_str_json_value_map_at_path(path)?
            .ok_or_else(|| {
                Error::StructureError(format!(
                    "unable to get borrowed str json value map property {path}"
                ))
            })
    }

    fn get_optional_hash256_bytes_at_path(&self, path: &str) -> Result<Option<[u8; 32]>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| v.to_hash256())
            .transpose()
    }

    fn get_hash256_bytes_at_path(&self, path: &str) -> Result<[u8; 32], Error> {
        self.get_optional_hash256_bytes_at_path(path)?
            .ok_or_else(|| Error::StructureError(format!("unable to get hash256 property {path}")))
    }

    fn get_optional_bytes_at_path(&self, path: &str) -> Result<Option<Vec<u8>>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| v.to_bytes())
            .transpose()
    }

    fn get_bytes_at_path(&self, path: &str) -> Result<Vec<u8>, Error> {
        self.get_optional_bytes_at_path(path)?.ok_or_else(|| {
            Error::StructureError(format!("unable to get system bytes property {path}"))
        })
    }

    fn get_optional_identifier_bytes_at_path(&self, path: &str) -> Result<Option<Vec<u8>>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| v.to_identifier_bytes())
            .transpose()
    }

    fn get_identifier_bytes_at_path(&self, path: &str) -> Result<Vec<u8>, Error> {
        self.get_optional_identifier_bytes_at_path(path)?
            .ok_or_else(|| {
                Error::StructureError(format!("unable to get system bytes property {path}"))
            })
    }

    fn get_optional_binary_bytes_at_path(&self, path: &str) -> Result<Option<Vec<u8>>, Error> {
        self.get_optional_at_path(path)?
            .map(|v| v.to_binary_bytes())
            .transpose()
    }

    fn get_binary_bytes_at_path(&self, path: &str) -> Result<Vec<u8>, Error> {
        self.get_optional_binary_bytes_at_path(path)?
            .ok_or_else(|| {
                Error::StructureError(format!("unable to get system bytes property {path}"))
            })
    }

    fn remove_optional_hash256_bytes_at_path(
        &mut self,
        path: &str,
    ) -> Result<Option<[u8; 32]>, Error> {
        self.remove(path)
            .map(|v| v.borrow().to_hash256())
            .transpose()
    }

    fn remove_hash256_bytes_at_path(&mut self, path: &str) -> Result<[u8; 32], Error> {
        self.remove_optional_hash256_bytes_at_path(path)?
            .ok_or_else(|| {
                Error::StructureError(format!("unable to remove hash256 property {path}"))
            })
    }

    fn remove_optional_identifier_at_path(
        &mut self,
        path: &str,
    ) -> Result<Option<Identifier>, Error> {
        self.remove(path)
            .map(|v| v.borrow().to_identifier())
            .transpose()
    }

    fn remove_identifier_at_path(&mut self, path: &str) -> Result<Identifier, Error> {
        self.remove_optional_identifier_at_path(path)?
            .ok_or_else(|| {
                Error::StructureError(format!("unable to remove system bytes property {path}"))
            })
    }

    fn remove_optional_string_at_path(&mut self, path: &str) -> Result<Option<String>, Error> {
        self.remove(path).map(|v| v.borrow().to_text()).transpose()
    }

    fn remove_string_at_path(&mut self, path: &str) -> Result<String, Error> {
        self.remove_optional_string_at_path(path)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove string property {path}"))
        })
    }

    fn remove_optional_float_at_path(&mut self, path: &str) -> Result<Option<f64>, Error> {
        self.remove(path)
            .and_then(|v| {
                let borrowed = v.borrow();
                if borrowed.is_null() {
                    None
                } else {
                    Some(v.borrow().to_float())
                }
            })
            .transpose()
    }

    fn remove_float_at_path(&mut self, path: &str) -> Result<f64, Error> {
        self.remove_optional_float_at_path(path)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove float property {path}")))
    }

    fn get_optional_float_at_path(&self, path: &str) -> Result<Option<f64>, Error> {
        self.get_optional_at_path(path)?
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_float())
                }
            })
            .transpose()
    }

    fn get_float_at_path(&self, path: &str) -> Result<f64, Error> {
        self.get_optional_float_at_path(path)?
            .ok_or_else(|| Error::StructureError(format!("unable to get float property {path}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Identifier, Value};
    use std::collections::BTreeMap;

    // -----------------------------------------------------------------------
    // Helper: build a BTreeMap<String, Value> from tuples
    // -----------------------------------------------------------------------

    fn map_with(entries: Vec<(&str, Value)>) -> BTreeMap<String, Value> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    /// Build a nested Value::Map from a BTreeMap-like structure
    fn nested_value_map(entries: Vec<(&str, Value)>) -> Value {
        Value::Map(
            entries
                .into_iter()
                .map(|(k, v)| (Value::Text(k.to_string()), v))
                .collect(),
        )
    }

    // -----------------------------------------------------------------------
    // get_at_path / get_optional_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_at_path_single_level() {
        let map = map_with(vec![("name", Value::Text("alice".to_string()))]);
        let result = map.get_at_path("name").unwrap();
        assert_eq!(result, &Value::Text("alice".to_string()));
    }

    #[test]
    fn get_at_path_nested() {
        let inner = nested_value_map(vec![("city", Value::Text("NYC".to_string()))]);
        let map = map_with(vec![("address", inner)]);
        let result = map.get_at_path("address.city").unwrap();
        assert_eq!(result, &Value::Text("NYC".to_string()));
    }

    #[test]
    fn get_at_path_deeply_nested() {
        let deep = nested_value_map(vec![("z", Value::U64(42))]);
        let mid = nested_value_map(vec![("y", deep)]);
        let map = map_with(vec![("x", mid)]);
        let result = map.get_at_path("x.y.z").unwrap();
        assert_eq!(result, &Value::U64(42));
    }

    #[test]
    fn get_at_path_missing_top_level_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.get_at_path("missing").is_err());
    }

    #[test]
    fn get_at_path_missing_nested_key_errors() {
        let inner = nested_value_map(vec![("a", Value::U64(1))]);
        let map = map_with(vec![("top", inner)]);
        assert!(map.get_at_path("top.nonexistent").is_err());
    }

    #[test]
    fn get_at_path_empty_path_errors() {
        let map = map_with(vec![("a", Value::U64(1))]);
        // Empty string splits into one element "", which won't match anything
        // The actual implementation: split("").next() returns Some("")
        // but get("") returns None => StructureError
        assert!(map.get_at_path("").is_err());
    }

    #[test]
    fn get_optional_at_path_returns_some() {
        let map = map_with(vec![("key", Value::Bool(true))]);
        let result = map.get_optional_at_path("key").unwrap();
        assert_eq!(result, Some(&Value::Bool(true)));
    }

    #[test]
    fn get_optional_at_path_returns_none_for_missing_top_level() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_at_path("missing").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_at_path_returns_none_for_missing_nested() {
        let inner = nested_value_map(vec![("a", Value::U64(1))]);
        let map = map_with(vec![("top", inner)]);
        let result = map.get_optional_at_path("top.missing").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_at_path_nested_returns_some() {
        let inner = nested_value_map(vec![("val", Value::Float(9.9))]);
        let map = map_with(vec![("nested", inner)]);
        let result = map.get_optional_at_path("nested.val").unwrap();
        assert_eq!(result, Some(&Value::Float(9.9)));
    }

    // -----------------------------------------------------------------------
    // get_identifier_at_path / get_optional_identifier_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_identifier_at_path_success() {
        let id = [1u8; 32];
        let map = map_with(vec![("id", Value::Bytes32(id))]);
        let result = map.get_identifier_at_path("id").unwrap();
        assert_eq!(result, id);
    }

    #[test]
    fn get_identifier_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.get_identifier_at_path("id").is_err());
    }

    #[test]
    fn get_optional_identifier_at_path_returns_some() {
        let id = [2u8; 32];
        let map = map_with(vec![("id", Value::Bytes32(id))]);
        let result = map.get_optional_identifier_at_path("id").unwrap();
        assert_eq!(result, Some(id));
    }

    #[test]
    fn get_optional_identifier_at_path_returns_none_for_missing() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_identifier_at_path("id").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // get_string_at_path / get_optional_string_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_string_at_path_success() {
        let map = map_with(vec![("s", Value::Text("hello".to_string()))]);
        let result = map.get_string_at_path("s").unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn get_string_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.get_string_at_path("s").is_err());
    }

    #[test]
    fn get_optional_string_at_path_returns_some() {
        let map = map_with(vec![("s", Value::Text("world".to_string()))]);
        let result = map.get_optional_string_at_path("s").unwrap();
        assert_eq!(result, Some("world".to_string()));
    }

    #[test]
    fn get_optional_string_at_path_returns_none() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_string_at_path("s").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_string_at_path_wrong_type_errors() {
        let map = map_with(vec![("s", Value::U64(42))]);
        assert!(map.get_optional_string_at_path("s").is_err());
    }

    // -----------------------------------------------------------------------
    // get_str_at_path / get_optional_str_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_str_at_path_success() {
        let map = map_with(vec![("s", Value::Text("borrow_me".to_string()))]);
        let result = map.get_str_at_path("s").unwrap();
        assert_eq!(result, "borrow_me");
    }

    #[test]
    fn get_str_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.get_str_at_path("s").is_err());
    }

    #[test]
    fn get_optional_str_at_path_returns_some() {
        let map = map_with(vec![("s", Value::Text("ref".to_string()))]);
        let result = map.get_optional_str_at_path("s").unwrap();
        assert_eq!(result, Some("ref"));
    }

    #[test]
    fn get_optional_str_at_path_returns_none() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_str_at_path("s").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // get_integer_at_path / get_optional_integer_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_integer_at_path_success() {
        let map = map_with(vec![("num", Value::U64(100))]);
        let result: u64 = map.get_integer_at_path("num").unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn get_integer_at_path_nested() {
        let inner = nested_value_map(vec![("val", Value::I32(-5))]);
        let map = map_with(vec![("data", inner)]);
        let result: i32 = map.get_integer_at_path("data.val").unwrap();
        assert_eq!(result, -5);
    }

    #[test]
    fn get_integer_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result: Result<u64, Error> = map.get_integer_at_path("num");
        assert!(result.is_err());
    }

    #[test]
    fn get_optional_integer_at_path_returns_some() {
        let map = map_with(vec![("n", Value::U32(7))]);
        let result: Option<u32> = map.get_optional_integer_at_path("n").unwrap();
        assert_eq!(result, Some(7));
    }

    #[test]
    fn get_optional_integer_at_path_returns_none_for_missing() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result: Option<u32> = map.get_optional_integer_at_path("n").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_integer_at_path_returns_none_for_null() {
        let map = map_with(vec![("n", Value::Null)]);
        let result: Option<u32> = map.get_optional_integer_at_path("n").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // get_float_at_path / get_optional_float_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_float_at_path_success() {
        let map = map_with(vec![("f", Value::Float(1.23))]);
        let result = map.get_float_at_path("f").unwrap();
        assert!((result - 1.23).abs() < f64::EPSILON);
    }

    #[test]
    fn get_float_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.get_float_at_path("f").is_err());
    }

    #[test]
    fn get_optional_float_at_path_returns_some() {
        let map = map_with(vec![("f", Value::Float(2.72))]);
        let result = map.get_optional_float_at_path("f").unwrap();
        assert_eq!(result, Some(2.72));
    }

    #[test]
    fn get_optional_float_at_path_returns_none_for_missing() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_float_at_path("f").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_float_at_path_returns_none_for_null() {
        let map = map_with(vec![("f", Value::Null)]);
        let result = map.get_optional_float_at_path("f").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // get_bool_at_path / get_optional_bool_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_bool_at_path_success() {
        let map = map_with(vec![("flag", Value::Bool(true))]);
        let result = map.get_bool_at_path("flag").unwrap();
        assert!(result);
    }

    #[test]
    fn get_bool_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.get_bool_at_path("flag").is_err());
    }

    #[test]
    fn get_optional_bool_at_path_returns_some() {
        let map = map_with(vec![("b", Value::Bool(false))]);
        let result = map.get_optional_bool_at_path("b").unwrap();
        assert_eq!(result, Some(false));
    }

    #[test]
    fn get_optional_bool_at_path_returns_none_for_missing() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_bool_at_path("b").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_bool_at_path_returns_none_for_null() {
        let map = map_with(vec![("b", Value::Null)]);
        let result = map.get_optional_bool_at_path("b").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // get_inner_value_array_at_path / get_optional_inner_value_array_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_inner_value_array_at_path_success() {
        let arr = Value::Array(vec![Value::U64(1), Value::U64(2), Value::U64(3)]);
        let map = map_with(vec![("arr", arr)]);
        let result: Vec<&Value> = map.get_inner_value_array_at_path("arr").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], &Value::U64(1));
    }

    #[test]
    fn get_inner_value_array_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result: Result<Vec<&Value>, Error> = map.get_inner_value_array_at_path("arr");
        assert!(result.is_err());
    }

    #[test]
    fn get_optional_inner_value_array_returns_some() {
        let arr = Value::Array(vec![Value::Bool(true)]);
        let map = map_with(vec![("arr", arr)]);
        let result: Option<Vec<&Value>> =
            map.get_optional_inner_value_array_at_path("arr").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn get_optional_inner_value_array_returns_none_for_missing() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result: Option<Vec<&Value>> =
            map.get_optional_inner_value_array_at_path("arr").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // get_inner_string_array_at_path / get_optional_inner_string_array_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_inner_string_array_at_path_success() {
        let arr = Value::Array(vec![
            Value::Text("a".to_string()),
            Value::Text("b".to_string()),
        ]);
        let map = map_with(vec![("names", arr)]);
        let result: Vec<String> = map.get_inner_string_array_at_path("names").unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn get_inner_string_array_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result: Result<Vec<String>, Error> = map.get_inner_string_array_at_path("names");
        assert!(result.is_err());
    }

    #[test]
    fn get_optional_inner_string_array_returns_some() {
        let arr = Value::Array(vec![Value::Text("x".to_string())]);
        let map = map_with(vec![("names", arr)]);
        let result: Option<Vec<String>> = map
            .get_optional_inner_string_array_at_path("names")
            .unwrap();
        assert_eq!(result, Some(vec!["x".to_string()]));
    }

    #[test]
    fn get_optional_inner_string_array_returns_none_for_missing() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result: Option<Vec<String>> = map
            .get_optional_inner_string_array_at_path("names")
            .unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_inner_string_array_errors_on_non_string_elements() {
        let arr = Value::Array(vec![Value::U64(42)]);
        let map = map_with(vec![("names", arr)]);
        let result: Result<Option<Vec<String>>, Error> =
            map.get_optional_inner_string_array_at_path("names");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // get_optional_inner_borrowed_map_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_optional_inner_borrowed_map_at_path_returns_some() {
        let inner = nested_value_map(vec![("k", Value::U64(1))]);
        let map = map_with(vec![("m", inner)]);
        let result = map.get_optional_inner_borrowed_map_at_path("m").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn get_optional_inner_borrowed_map_at_path_returns_none_for_missing() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_inner_borrowed_map_at_path("m").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn get_optional_inner_borrowed_map_at_path_errors_on_non_map() {
        let map = map_with(vec![("m", Value::U64(42))]);
        assert!(map.get_optional_inner_borrowed_map_at_path("m").is_err());
    }

    // -----------------------------------------------------------------------
    // get_inner_borrowed_str_value_map_at_path /
    // get_optional_inner_borrowed_str_value_map_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_inner_borrowed_str_value_map_at_path_success() {
        let inner = nested_value_map(vec![("k", Value::Text("v".to_string()))]);
        let map = map_with(vec![("m", inner)]);
        let result: BTreeMap<String, &Value> =
            map.get_inner_borrowed_str_value_map_at_path("m").unwrap();
        assert_eq!(result.get("k"), Some(&&Value::Text("v".to_string())));
    }

    #[test]
    fn get_inner_borrowed_str_value_map_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result: Result<BTreeMap<String, &Value>, Error> =
            map.get_inner_borrowed_str_value_map_at_path("m");
        assert!(result.is_err());
    }

    #[test]
    fn get_optional_inner_borrowed_str_value_map_returns_some() {
        let inner = nested_value_map(vec![("a", Value::Bool(true))]);
        let map = map_with(vec![("m", inner)]);
        let result: Option<BTreeMap<String, &Value>> = map
            .get_optional_inner_borrowed_str_value_map_at_path("m")
            .unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn get_optional_inner_borrowed_str_value_map_returns_none_for_missing() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result: Option<BTreeMap<String, &Value>> = map
            .get_optional_inner_borrowed_str_value_map_at_path("m")
            .unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // get_hash256_bytes_at_path / get_optional_hash256_bytes_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_hash256_bytes_at_path_success() {
        let hash = [5u8; 32];
        let map = map_with(vec![("h", Value::Bytes32(hash))]);
        let result = map.get_hash256_bytes_at_path("h").unwrap();
        assert_eq!(result, hash);
    }

    #[test]
    fn get_hash256_bytes_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.get_hash256_bytes_at_path("h").is_err());
    }

    #[test]
    fn get_optional_hash256_bytes_at_path_returns_some() {
        let hash = [6u8; 32];
        let map = map_with(vec![("h", Value::Bytes32(hash))]);
        let result = map.get_optional_hash256_bytes_at_path("h").unwrap();
        assert_eq!(result, Some(hash));
    }

    #[test]
    fn get_optional_hash256_bytes_at_path_returns_none() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_hash256_bytes_at_path("h").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // get_bytes_at_path / get_optional_bytes_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_bytes_at_path_success() {
        let bytes = vec![1, 2, 3];
        let map = map_with(vec![("b", Value::Bytes(bytes.clone()))]);
        let result = map.get_bytes_at_path("b").unwrap();
        assert_eq!(result, bytes);
    }

    #[test]
    fn get_bytes_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.get_bytes_at_path("b").is_err());
    }

    #[test]
    fn get_optional_bytes_at_path_returns_some() {
        let bytes = vec![4, 5];
        let map = map_with(vec![("b", Value::Bytes(bytes.clone()))]);
        let result = map.get_optional_bytes_at_path("b").unwrap();
        assert_eq!(result, Some(bytes));
    }

    #[test]
    fn get_optional_bytes_at_path_returns_none() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_bytes_at_path("b").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // get_identifier_bytes_at_path / get_optional_identifier_bytes_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_identifier_bytes_at_path_success() {
        let id = [7u8; 32];
        let map = map_with(vec![("id", Value::Identifier(id))]);
        let result = map.get_identifier_bytes_at_path("id").unwrap();
        assert_eq!(result, id.to_vec());
    }

    #[test]
    fn get_identifier_bytes_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.get_identifier_bytes_at_path("id").is_err());
    }

    #[test]
    fn get_optional_identifier_bytes_at_path_returns_some() {
        let id = [8u8; 32];
        let map = map_with(vec![("id", Value::Identifier(id))]);
        let result = map.get_optional_identifier_bytes_at_path("id").unwrap();
        assert_eq!(result, Some(id.to_vec()));
    }

    #[test]
    fn get_optional_identifier_bytes_at_path_returns_none() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_identifier_bytes_at_path("id").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // get_binary_bytes_at_path / get_optional_binary_bytes_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn get_binary_bytes_at_path_success() {
        let bytes = vec![10, 20, 30];
        let map = map_with(vec![("bin", Value::Bytes(bytes.clone()))]);
        let result = map.get_binary_bytes_at_path("bin").unwrap();
        assert_eq!(result, bytes);
    }

    #[test]
    fn get_binary_bytes_at_path_missing_errors() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.get_binary_bytes_at_path("bin").is_err());
    }

    #[test]
    fn get_optional_binary_bytes_at_path_returns_some() {
        let bytes = vec![40, 50];
        let map = map_with(vec![("bin", Value::Bytes(bytes.clone()))]);
        let result = map.get_optional_binary_bytes_at_path("bin").unwrap();
        assert_eq!(result, Some(bytes));
    }

    #[test]
    fn get_optional_binary_bytes_at_path_returns_none() {
        let map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.get_optional_binary_bytes_at_path("bin").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // remove_string_at_path / remove_optional_string_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn remove_string_at_path_success() {
        let mut map = map_with(vec![("name", Value::Text("test".to_string()))]);
        let result = map.remove_string_at_path("name").unwrap();
        assert_eq!(result, "test");
        assert!(map.is_empty());
    }

    #[test]
    fn remove_string_at_path_missing_errors() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.remove_string_at_path("name").is_err());
    }

    #[test]
    fn remove_optional_string_at_path_returns_some() {
        let mut map = map_with(vec![("s", Value::Text("hi".to_string()))]);
        let result = map.remove_optional_string_at_path("s").unwrap();
        assert_eq!(result, Some("hi".to_string()));
    }

    #[test]
    fn remove_optional_string_at_path_returns_none() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.remove_optional_string_at_path("s").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // remove_float_at_path / remove_optional_float_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn remove_float_at_path_success() {
        let mut map = map_with(vec![("f", Value::Float(1.5))]);
        let result = map.remove_float_at_path("f").unwrap();
        assert!((result - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn remove_float_at_path_missing_errors() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.remove_float_at_path("f").is_err());
    }

    #[test]
    fn remove_optional_float_at_path_returns_some() {
        let mut map = map_with(vec![("f", Value::Float(0.5))]);
        let result = map.remove_optional_float_at_path("f").unwrap();
        assert_eq!(result, Some(0.5));
    }

    #[test]
    fn remove_optional_float_at_path_returns_none_for_missing() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.remove_optional_float_at_path("f").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn remove_optional_float_at_path_returns_none_for_null() {
        let mut map = map_with(vec![("f", Value::Null)]);
        let result = map.remove_optional_float_at_path("f").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // remove_integer_at_path / remove_optional_integer_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn remove_integer_at_path_success() {
        let mut map = map_with(vec![("n", Value::U64(99))]);
        let result: u64 = map.remove_integer_at_path("n").unwrap();
        assert_eq!(result, 99);
    }

    #[test]
    fn remove_integer_at_path_missing_errors() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        let result: Result<u64, Error> = map.remove_integer_at_path("n");
        assert!(result.is_err());
    }

    #[test]
    fn remove_optional_integer_at_path_returns_some() {
        let mut map = map_with(vec![("n", Value::I32(-3))]);
        let result: Option<i32> = map.remove_optional_integer_at_path("n").unwrap();
        assert_eq!(result, Some(-3));
    }

    #[test]
    fn remove_optional_integer_at_path_returns_none_for_missing() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        let result: Option<i32> = map.remove_optional_integer_at_path("n").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn remove_optional_integer_at_path_returns_none_for_null() {
        let mut map = map_with(vec![("n", Value::Null)]);
        let result: Option<i32> = map.remove_optional_integer_at_path("n").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // remove_hash256_bytes_at_path / remove_optional_hash256_bytes_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn remove_hash256_bytes_at_path_success() {
        let hash = [9u8; 32];
        let mut map = map_with(vec![("h", Value::Bytes32(hash))]);
        let result = map.remove_hash256_bytes_at_path("h").unwrap();
        assert_eq!(result, hash);
        assert!(map.is_empty());
    }

    #[test]
    fn remove_hash256_bytes_at_path_missing_errors() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.remove_hash256_bytes_at_path("h").is_err());
    }

    #[test]
    fn remove_optional_hash256_bytes_at_path_returns_some() {
        let hash = [10u8; 32];
        let mut map = map_with(vec![("h", Value::Bytes32(hash))]);
        let result = map.remove_optional_hash256_bytes_at_path("h").unwrap();
        assert_eq!(result, Some(hash));
    }

    #[test]
    fn remove_optional_hash256_bytes_at_path_returns_none() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.remove_optional_hash256_bytes_at_path("h").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // remove_identifier_at_path / remove_optional_identifier_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn remove_identifier_at_path_success() {
        let id = [11u8; 32];
        let mut map = map_with(vec![("id", Value::Identifier(id))]);
        let result = map.remove_identifier_at_path("id").unwrap();
        assert_eq!(result, Identifier::from(id));
        assert!(map.is_empty());
    }

    #[test]
    fn remove_identifier_at_path_missing_errors() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        assert!(map.remove_identifier_at_path("id").is_err());
    }

    #[test]
    fn remove_optional_identifier_at_path_returns_some() {
        let id = [12u8; 32];
        let mut map = map_with(vec![("id", Value::Identifier(id))]);
        let result = map.remove_optional_identifier_at_path("id").unwrap();
        assert_eq!(result, Some(Identifier::from(id)));
    }

    #[test]
    fn remove_optional_identifier_at_path_returns_none() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        let result = map.remove_optional_identifier_at_path("id").unwrap();
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // Nested path traversal edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn get_at_path_works_with_ref_values() {
        // Test that the impl works for BTreeMap<String, &Value> via the generic impl
        let source: BTreeMap<String, Value> =
            map_with(vec![("key", Value::Text("val".to_string()))]);
        let ref_map: BTreeMap<String, &Value> =
            source.iter().map(|(k, v)| (k.clone(), v)).collect();
        let result = ref_map.get_at_path("key").unwrap();
        assert_eq!(result, &Value::Text("val".to_string()));
    }

    #[test]
    fn get_at_path_ref_map_nested() {
        let inner = nested_value_map(vec![("deep", Value::Bool(true))]);
        let source = map_with(vec![("top", inner)]);
        let ref_map: BTreeMap<String, &Value> =
            source.iter().map(|(k, v)| (k.clone(), v)).collect();
        let result = ref_map.get_at_path("top.deep").unwrap();
        assert_eq!(result, &Value::Bool(true));
    }

    // -----------------------------------------------------------------------
    // Nested path: multi-level structure tests
    // -----------------------------------------------------------------------

    #[test]
    fn get_string_at_nested_path() {
        let level2 = nested_value_map(vec![("name", Value::Text("deep".to_string()))]);
        let level1 = nested_value_map(vec![("child", level2)]);
        let map = map_with(vec![("parent", level1)]);
        let result = map.get_string_at_path("parent.child.name").unwrap();
        assert_eq!(result, "deep");
    }

    #[test]
    fn get_bool_at_nested_path() {
        let inner = nested_value_map(vec![("enabled", Value::Bool(false))]);
        let map = map_with(vec![("config", inner)]);
        let result = map.get_bool_at_path("config.enabled").unwrap();
        assert!(!result);
    }
}
