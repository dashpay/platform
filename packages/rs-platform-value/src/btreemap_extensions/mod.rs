#[cfg(feature = "json")]
use serde_json::Value as JsonValue;
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::convert::TryFrom;
#[cfg(feature = "json")]
use std::convert::TryInto;
use std::iter::FromIterator;

use crate::{BinaryData, Error, Identifier, Value, ValueMap};

pub(crate) mod btreemap_field_replacement;
mod btreemap_mut_value_extensions;
mod btreemap_path_extensions;
mod btreemap_path_insertion_extensions;
mod btreemap_removal_extensions;
mod btreemap_removal_inner_value_extensions;

pub use btreemap_field_replacement::BTreeValueMapReplacementPathHelper;
pub use btreemap_mut_value_extensions::BTreeMutValueMapHelper;
pub use btreemap_path_extensions::BTreeValueMapPathHelper;
pub use btreemap_path_insertion_extensions::BTreeValueMapInsertionPathHelper;
pub use btreemap_removal_extensions::BTreeValueRemoveFromMapHelper;
pub use btreemap_removal_extensions::BTreeValueRemoveTupleFromMapHelper;
pub use btreemap_removal_inner_value_extensions::BTreeValueRemoveInnerValueFromMapHelper;

pub trait BTreeValueMapHelper {
    fn get_optional_identifier(&self, key: &str) -> Result<Option<Identifier>, Error>;
    fn get_identifier(&self, key: &str) -> Result<Identifier, Error>;
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
    fn get_optional_inner_map_in_array<
        'a,
        M: FromIterator<(String, &'a Value)>,
        I: FromIterator<M>,
    >(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_map_in_array<'a, M: FromIterator<(String, &'a Value)>, I: FromIterator<M>>(
        &'a self,
        key: &str,
    ) -> Result<I, Error>;
    fn get_optional_inner_string_array<I: FromIterator<String>>(
        &self,
        key: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_string_array<I: FromIterator<String>>(&self, key: &str) -> Result<I, Error>;
    fn get_optional_map(&self, key: &str) -> Result<Option<&Vec<(Value, Value)>>, Error>;
    fn get_optional_str_value_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_borrowed_str_value_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        key: &str,
    ) -> Result<I, Error>;
    #[cfg(feature = "json")]
    fn get_optional_inner_str_json_value_map<I: FromIterator<(String, JsonValue)>>(
        &self,
        key: &str,
    ) -> Result<Option<I>, Error>;
    #[cfg(feature = "json")]
    fn get_inner_str_json_value_map<I: FromIterator<(String, JsonValue)>>(
        &self,
        key: &str,
    ) -> Result<I, Error>;
    fn get_optional_hash256_bytes(&self, key: &str) -> Result<Option<[u8; 32]>, Error>;
    fn get_hash256_bytes(&self, key: &str) -> Result<[u8; 32], Error>;
    fn get_optional_identifier_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, Error>;
    fn get_identifier_bytes(&self, key: &str) -> Result<Vec<u8>, Error>;
    fn get_optional_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, Error>;
    fn get_bytes(&self, key: &str) -> Result<Vec<u8>, Error>;
    fn get_optional_binary_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, Error>;
    fn get_binary_bytes(&self, key: &str) -> Result<Vec<u8>, Error>;
    fn get_optional_binary_data(&self, key: &str) -> Result<Option<BinaryData>, Error>;
    fn get_binary_data(&self, key: &str) -> Result<BinaryData, Error>;
    fn get_optional_u64(&self, key: &str) -> Result<Option<u64>, Error>;
    fn get_u64(&self, key: &str) -> Result<u64, Error>;
}

impl<V> BTreeValueMapHelper for BTreeMap<String, V>
where
    V: Borrow<Value>,
{
    fn get_optional_identifier(&self, key: &str) -> Result<Option<Identifier>, Error> {
        self.get(key)
            .map(|v| v.borrow().to_identifier())
            .transpose()
    }

    fn get_identifier(&self, key: &str) -> Result<Identifier, Error> {
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
        self.get(key)
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

    fn get_optional_bool(&self, key: &str) -> Result<Option<bool>, Error> {
        self.get(key)
            .and_then(|v| {
                let borrowed = v.borrow();
                if borrowed.is_null() {
                    None
                } else {
                    Some(v.borrow().to_bool())
                }
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

    fn get_optional_inner_map_in_array<
        'a,
        M: FromIterator<(String, &'a Value)>,
        I: FromIterator<M>,
    >(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, Error> {
        self.get(key)
            .map(|v| {
                v.borrow()
                    .as_array()
                    .map(|vec| {
                        vec.iter()
                            .map(|v| v.to_ref_string_map::<M>())
                            .collect::<Result<I, Error>>()
                    })
                    .ok_or_else(|| Error::StructureError(format!("{key} must be a an array")))
            })
            .transpose()?
            .transpose()
    }

    fn get_inner_map_in_array<'a, M: FromIterator<(String, &'a Value)>, I: FromIterator<M>>(
        &'a self,
        key: &str,
    ) -> Result<I, Error> {
        self.get_optional_inner_map_in_array(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to get inner value array property {key}"))
        })
    }

    fn get_optional_inner_string_array<I: FromIterator<String>>(
        &self,
        key: &str,
    ) -> Result<Option<I>, Error> {
        self.get(key)
            .and_then(|v| {
                let value = v.borrow();
                if value.is_null() {
                    None
                } else {
                    Some(value.to_array_ref().and_then(|inner| {
                        inner
                            .iter()
                            .map(|v| {
                                let Some(str) = v.as_text() else {
                                    return Err(Error::StructureError(format!(
                                        "{key} must be an string"
                                    )));
                                };
                                Ok(str.to_string())
                            })
                            .collect::<Result<I, Error>>()
                    }))
                }
            })
            .transpose()
    }

    fn get_inner_string_array<I: FromIterator<String>>(&self, key: &str) -> Result<I, Error> {
        self.get_optional_inner_string_array(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to get inner string property {key}"))
        })
    }

    fn get_optional_map(&self, key: &str) -> Result<Option<&ValueMap>, Error> {
        self.get(key)
            .and_then(|v| {
                let value = v.borrow();
                if value.is_null() {
                    None
                } else {
                    Some(
                        value
                            .as_map()
                            .ok_or_else(|| Error::StructureError(format!("{key} must be a map"))),
                    )
                }
            })
            .transpose()
    }

    fn get_optional_str_value_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        key: &str,
    ) -> Result<Option<I>, Error> {
        self.get(key)
            .and_then(|v| {
                let value = v.borrow();
                if value.is_null() {
                    None
                } else {
                    Some(value.to_map_ref().and_then(|inner| {
                        inner
                            .iter()
                            .map(|(k, v)| Ok((k.to_text()?, v)))
                            .collect::<Result<I, Error>>()
                    }))
                }
            })
            .transpose()
    }

    fn get_inner_borrowed_str_value_map<'a, I: FromIterator<(String, &'a Value)>>(
        &'a self,
        key: &str,
    ) -> Result<I, Error> {
        self.get_optional_str_value_map(key)?.ok_or_else(|| {
            Error::StructureError(format!(
                "unable to get borrowed str value map property {key}"
            ))
        })
    }

    #[cfg(feature = "json")]
    fn get_optional_inner_str_json_value_map<I: FromIterator<(String, JsonValue)>>(
        &self,
        key: &str,
    ) -> Result<Option<I>, Error> {
        self.get(key)
            .and_then(|v| {
                let value = v.borrow();
                if value.is_null() {
                    None
                } else {
                    Some(value.to_map_ref().and_then(|inner| {
                        inner
                            .iter()
                            .map(|(k, v)| Ok((k.to_text()?, v.clone().try_into()?)))
                            .collect::<Result<I, Error>>()
                    }))
                }
            })
            .transpose()
    }

    #[cfg(feature = "json")]
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

    fn get_optional_hash256_bytes(&self, key: &str) -> Result<Option<[u8; 32]>, Error> {
        self.get(key).map(|v| v.borrow().to_hash256()).transpose()
    }

    fn get_hash256_bytes(&self, key: &str) -> Result<[u8; 32], Error> {
        self.get_optional_hash256_bytes(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get hash256 property {key}")))
    }

    fn get_optional_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.get(key).map(|v| v.borrow().to_bytes()).transpose()
    }

    fn get_bytes(&self, key: &str) -> Result<Vec<u8>, Error> {
        self.get_optional_bytes(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get bytes property {key}")))
    }

    fn get_optional_identifier_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.get(key)
            .map(|v| v.borrow().to_identifier_bytes())
            .transpose()
    }

    fn get_identifier_bytes(&self, key: &str) -> Result<Vec<u8>, Error> {
        self.get_optional_identifier_bytes(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get bytes property {key}")))
    }

    fn get_optional_binary_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.get(key)
            .map(|v| v.borrow().to_binary_bytes())
            .transpose()
    }

    fn get_binary_bytes(&self, key: &str) -> Result<Vec<u8>, Error> {
        self.get_optional_binary_bytes(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get bytes property {key}")))
    }

    fn get_optional_binary_data(&self, key: &str) -> Result<Option<BinaryData>, Error> {
        self.get(key)
            .map(|v| v.borrow().to_binary_data())
            .transpose()
    }

    fn get_binary_data(&self, key: &str) -> Result<BinaryData, Error> {
        self.get_optional_binary_data(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to get binary data property {key}"))
        })
    }

    fn get_optional_float(&self, key: &str) -> Result<Option<f64>, Error> {
        self.get(key)
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

    fn get_float(&self, key: &str) -> Result<f64, Error> {
        self.get_optional_float(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get float property {key}")))
    }

    fn get_optional_u64(&self, key: &str) -> Result<Option<u64>, Error> {
        Ok(self.get(key).and_then(|v| {
            let borrowed = v.borrow();
            if borrowed.is_null() {
                None
            } else {
                v.borrow().as_integer::<u64>()
            }
        }))
    }

    fn get_u64(&self, key: &str) -> Result<u64, Error> {
        self.get_optional_u64(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to get u64 property {key}")))
    }
}
