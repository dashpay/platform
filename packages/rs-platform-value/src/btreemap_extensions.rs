use serde_json::Value as JsonValue;
use std::borrow::Borrow;
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::{collections::BTreeMap, convert::TryInto};

use crate::{Error, Value};

pub trait BTreeValueMapHelper {
    fn get_optional_identifier(&self, key: &str) -> Result<Option<[u8; 32]>, Error>;
    fn get_identifier(&self, key: &str) -> Result<[u8; 32], Error>;
    fn get_optional_string(&self, key: &str) -> Result<Option<String>, Error>;
    fn get_string(&self, key: &str) -> Result<String, Error>;
    fn get_optional_str(&self, key: &str) -> Result<Option<&str>, Error>;
    fn get_str(&self, key: &str) -> Result<&str, Error>;
    fn get_optional_float(&self, key: &str) -> Result<Option<f64>, Error>;
    fn get_float(&self, key: &str) -> Result<f64, Error>;
    fn get_optional_integer<T>(&self, key: &str) -> Result<Option<T>, Error>
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
    fn get_integer<T>(&self, key: &str) -> Result<T, Error>
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
    fn get_optional_bool(&self, key: &str) -> Result<Option<bool>, Error>;
    fn get_bool(&self, key: &str) -> Result<bool, Error>;
    fn get_optional_inner_value_array<'a, I: FromIterator<&'a Value>>(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_value_array<'a, I: FromIterator<&'a Value>>(
        &'a self,
        key: &str,
    ) -> Result<I, Error>;
    fn get_optional_inner_string_array<I: FromIterator<String>>(
        &self,
        key: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_string_array<I: FromIterator<String>>(&self, key: &str) -> Result<I, Error>;
    fn get_optional_inner_borrowed_map(
        &self,
        key: &str,
    ) -> Result<Option<&Vec<(Value, Value)>>, Error>;
    fn get_optional_inner_borrowed_str_value_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_borrowed_str_value_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        key: &str,
    ) -> Result<I, Error>;
    fn get_optional_inner_str_json_value_map<I: FromIterator<(String, JsonValue)>>(
        &self,
        key: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_str_json_value_map<I: FromIterator<(String, JsonValue)>>(
        &self,
        key: &str,
    ) -> Result<I, Error>;
    fn get_optional_system_hash256_bytes(&self, key: &str) -> Result<Option<[u8; 32]>, Error>;
    fn get_system_hash256_bytes(&self, key: &str) -> Result<[u8; 32], Error>;
    fn get_optional_system_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, Error>;
    fn get_system_bytes(&self, key: &str) -> Result<Vec<u8>, Error>;
    fn remove_optional_string(&mut self, key: &str) -> Result<Option<String>, Error>;
    fn remove_string(&mut self, key: &str) -> Result<String, Error>;
    fn remove_optional_float(&mut self, key: &str) -> Result<Option<f64>, Error>;
    fn remove_float(&mut self, key: &str) -> Result<f64, Error>;
    fn remove_optional_integer<T>(&mut self, key: &str) -> Result<Option<T>, Error>
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
    fn remove_integer<T>(&mut self, key: &str) -> Result<T, Error>
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
    fn remove_optional_system_hash256_bytes(
        &mut self,
        key: &str,
    ) -> Result<Option<[u8; 32]>, Error>;
    fn remove_system_hash256_bytes(&mut self, key: &str) -> Result<[u8; 32], Error>;
    fn remove_optional_system_bytes(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error>;
    fn remove_system_bytes(&mut self, key: &str) -> Result<Vec<u8>, Error>;
}

impl<V> BTreeValueMapHelper for BTreeMap<String, V>
where
    V: Borrow<Value>,
{
    fn get_optional_identifier(&self, key: &str) -> Result<Option<[u8; 32]>, Error> {
        self.get(key)
            .map(|v| v.borrow().to_system_hash256())
            .transpose()
    }

    fn get_identifier(&self, key: &str) -> Result<[u8; 32], Error> {
        self.get_optional_identifier(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to get identifier property {key}"))
        })
    }

    fn get_optional_string(&self, key: &str) -> Result<Option<String>, Error> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_text()
                    .map(|str| str.to_string())
                    .ok_or_else(|| Error::StructureError(format!("{key} must be a string")))
            })
            .transpose()
    }

    fn get_string(&self, key: &str) -> Result<String, Error> {
        self.get_optional_string(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get string property {key}")))
    }

    fn get_optional_str(&self, key: &str) -> Result<Option<&str>, Error> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_text()
                    .ok_or_else(|| Error::StructureError(format!("{key} must be a string")))
            })
            .transpose()
    }

    fn get_str(&self, key: &str) -> Result<&str, Error> {
        self.get_optional_str(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get str property {key}")))
    }

    fn get_optional_integer<T>(&self, key: &str) -> Result<Option<T>, Error>
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
        self.get(key).map(|v| v.borrow().to_integer()).transpose()
    }

    fn get_integer<T>(&self, key: &str) -> Result<T, Error>
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
        self.get_optional_integer(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get integer property {key}")))
    }

    fn remove_optional_integer<T>(&mut self, key: &str) -> Result<Option<T>, Error>
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
        self.remove(key)
            .map(|v| v.borrow().to_integer())
            .transpose()
    }

    fn remove_integer<T>(&mut self, key: &str) -> Result<T, Error>
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
        self.remove_optional_integer(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove integer property {key}"))
        })
    }

    fn get_optional_bool(&self, key: &str) -> Result<Option<bool>, Error> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_bool()
                    .ok_or_else(|| Error::StructureError(format!("{key} must be a bool")))
            })
            .transpose()
    }

    fn get_bool(&self, key: &str) -> Result<bool, Error> {
        self.get_optional_bool(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get bool property {key}")))
    }

    fn get_optional_inner_value_array<'a, I: FromIterator<&'a Value>>(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, Error> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_array()
                    .map(|vec| vec.iter().collect())
                    .ok_or_else(|| Error::StructureError(format!("{key} must be a bool")))
            })
            .transpose()
    }

    fn get_inner_value_array<'a, I: FromIterator<&'a Value>>(
        &'a self,
        key: &str,
    ) -> Result<I, Error> {
        self.get_optional_inner_value_array(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to get inner value array property {key}"))
        })
    }

    fn get_optional_inner_string_array<I: FromIterator<String>>(
        &self,
        key: &str,
    ) -> Result<Option<I>, Error> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_array()
                    .map(|inner| {
                        inner
                            .iter()
                            .map(|v| {
                                let Some(str) = v.as_text() else {
                                    return Err(Error::StructureError(format!("{key} must be an string")))
                                };
                                Ok(str.to_string())
                            })
                            .collect::<Result<I, Error>>()
                    })
                    .transpose()?
                    .ok_or_else(|| Error::StructureError(format!("{key} must be a bool")))
            })
            .transpose()
    }

    fn get_inner_string_array<I: FromIterator<String>>(&self, key: &str) -> Result<I, Error> {
        self.get_optional_inner_string_array(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to get inner string property {key}"))
        })
    }

    fn get_optional_inner_borrowed_map(
        &self,
        key: &str,
    ) -> Result<Option<&Vec<(Value, Value)>>, Error> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_map()
                    .ok_or_else(|| Error::StructureError(format!("{key} must be a map")))
            })
            .transpose()
    }

    fn get_optional_inner_borrowed_str_value_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, Error> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_map()
                    .map(|inner| {
                        inner
                            .iter()
                            .map(|(k, v)| Ok((k.to_text()?, v)))
                            .collect::<Result<I, Error>>()
                    })
                    .transpose()?
                    .ok_or_else(|| Error::StructureError(format!("{key} must be a bool")))
            })
            .transpose()
    }

    fn get_inner_borrowed_str_value_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        key: &str,
    ) -> Result<I, Error> {
        self.get_optional_inner_borrowed_str_value_map(key)?
            .ok_or_else(|| {
                Error::StructureError(format!(
                    "unable to get borrowed str value map property {key}"
                ))
            })
    }

    fn get_optional_inner_str_json_value_map<I: FromIterator<(String, JsonValue)>>(
        &self,
        key: &str,
    ) -> Result<Option<I>, Error> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_map()
                    .map(|inner| {
                        inner
                            .iter()
                            .map(|(k, v)| Ok((k.to_text()?, v.clone().try_into()?)))
                            .collect::<Result<I, Error>>()
                    })
                    .transpose()?
                    .ok_or_else(|| Error::StructureError(format!("{key} must be a bool")))
            })
            .transpose()
    }

    fn get_inner_str_json_value_map<I: FromIterator<(String, JsonValue)>>(
        &self,
        key: &str,
    ) -> Result<I, Error> {
        self.get_optional_inner_str_json_value_map(key)?
            .ok_or_else(|| {
                Error::StructureError(format!(
                    "unable to get borrowed str json value map property {key}"
                ))
            })
    }

    fn get_optional_system_hash256_bytes(&self, key: &str) -> Result<Option<[u8; 32]>, Error> {
        self.get(key)
            .map(|v| v.borrow().to_system_hash256())
            .transpose()
    }

    fn get_system_hash256_bytes(&self, key: &str) -> Result<[u8; 32], Error> {
        self.get_optional_system_hash256_bytes(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to get system hash256 property {key}"))
        })
    }

    fn get_optional_system_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.get(key)
            .map(|v| v.borrow().to_system_bytes())
            .transpose()
    }

    fn get_system_bytes(&self, key: &str) -> Result<Vec<u8>, Error> {
        self.get_optional_system_bytes(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to get system bytes property {key}"))
        })
    }

    fn remove_optional_system_hash256_bytes(
        &mut self,
        key: &str,
    ) -> Result<Option<[u8; 32]>, Error> {
        self.remove(key)
            .map(|v| v.borrow().to_system_hash256())
            .transpose()
    }

    fn remove_system_hash256_bytes(&mut self, key: &str) -> Result<[u8; 32], Error> {
        self.remove_optional_system_hash256_bytes(key)?
            .ok_or_else(|| {
                Error::StructureError(format!("unable to remove system hash256 property {key}"))
            })
    }

    fn remove_optional_system_bytes(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.remove(key)
            .map(|v| v.borrow().to_system_bytes())
            .transpose()
    }

    fn remove_system_bytes(&mut self, key: &str) -> Result<Vec<u8>, Error> {
        self.remove_optional_system_bytes(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove system bytes property {key}"))
        })
    }

    fn remove_optional_string(&mut self, key: &str) -> Result<Option<String>, Error> {
        self.remove(key).map(|v| v.borrow().to_text()).transpose()
    }

    fn remove_string(&mut self, key: &str) -> Result<String, Error> {
        self.remove_optional_string(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove string property {key}")))
    }

    fn remove_optional_float(&mut self, key: &str) -> Result<Option<f64>, Error> {
        self.remove(key).map(|v| v.borrow().to_float()).transpose()
    }

    fn remove_float(&mut self, key: &str) -> Result<f64, Error> {
        self.remove_optional_float(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove float property {key}")))
    }

    fn get_optional_float(&self, key: &str) -> Result<Option<f64>, Error> {
        self.get(key).map(|v| v.borrow().to_float()).transpose()
    }

    fn get_float(&self, key: &str) -> Result<f64, Error> {
        self.get_optional_float(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get float property {key}")))
    }
}
