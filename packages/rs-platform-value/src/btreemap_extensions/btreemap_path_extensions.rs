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
