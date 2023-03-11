use crate::{Error, Value};
use std::collections::BTreeMap;

pub trait BTreeValueRemoveFromMapHelper {
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
    fn remove_optional_hash256_bytes(&mut self, key: &str) -> Result<Option<[u8; 32]>, Error>;
    fn remove_hash256_bytes(&mut self, key: &str) -> Result<[u8; 32], Error>;
    fn remove_optional_bytes(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error>;
    fn remove_bytes(&mut self, key: &str) -> Result<Vec<u8>, Error>;
    fn remove_optional_bool(&mut self, key: &str) -> Result<Option<bool>, Error>;
    fn remove_bool(&mut self, key: &str) -> Result<bool, Error>;
}

impl BTreeValueRemoveFromMapHelper for BTreeMap<String, &Value> {
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
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_integer())
                }
            })
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

    fn remove_optional_hash256_bytes(&mut self, key: &str) -> Result<Option<[u8; 32]>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_hash256())
                }
            })
            .transpose()
    }

    fn remove_hash256_bytes(&mut self, key: &str) -> Result<[u8; 32], Error> {
        self.remove_optional_hash256_bytes(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove hash256 property {key}"))
        })
    }

    fn remove_optional_bytes(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_identifier_bytes())
                }
            })
            .transpose()
    }

    fn remove_bytes(&mut self, key: &str) -> Result<Vec<u8>, Error> {
        self.remove_optional_bytes(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove bytes property {key}")))
    }

    fn remove_optional_string(&mut self, key: &str) -> Result<Option<String>, Error> {
        self.remove(key)
            .and_then(|v| if v.is_null() { None } else { Some(v.to_text()) })
            .transpose()
    }

    fn remove_string(&mut self, key: &str) -> Result<String, Error> {
        self.remove_optional_string(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove string property {key}")))
    }

    fn remove_optional_float(&mut self, key: &str) -> Result<Option<f64>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_float())
                }
            })
            .transpose()
    }

    fn remove_float(&mut self, key: &str) -> Result<f64, Error> {
        self.remove_optional_float(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove float property {key}")))
    }

    fn remove_optional_bool(&mut self, key: &str) -> Result<Option<bool>, Error> {
        self.remove(key)
            .and_then(|v| if v.is_null() { None } else { Some(v.to_bool()) })
            .transpose()
    }

    fn remove_bool(&mut self, key: &str) -> Result<bool, Error> {
        self.remove_optional_bool(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove float property {key}")))
    }
}

impl BTreeValueRemoveFromMapHelper for BTreeMap<String, Value> {
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
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_integer())
                }
            })
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

    fn remove_optional_hash256_bytes(&mut self, key: &str) -> Result<Option<[u8; 32]>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_hash256())
                }
            })
            .transpose()
    }

    fn remove_hash256_bytes(&mut self, key: &str) -> Result<[u8; 32], Error> {
        self.remove_optional_hash256_bytes(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove hash256 property {key}"))
        })
    }

    fn remove_optional_bytes(&mut self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_identifier_bytes())
                }
            })
            .transpose()
    }

    fn remove_bytes(&mut self, key: &str) -> Result<Vec<u8>, Error> {
        self.remove_optional_bytes(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove bytes property {key}")))
    }

    fn remove_optional_string(&mut self, key: &str) -> Result<Option<String>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_text())
                }
            })
            .transpose()
    }

    fn remove_string(&mut self, key: &str) -> Result<String, Error> {
        self.remove_optional_string(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove string property {key}")))
    }

    fn remove_optional_float(&mut self, key: &str) -> Result<Option<f64>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_float())
                }
            })
            .transpose()
    }

    fn remove_float(&mut self, key: &str) -> Result<f64, Error> {
        self.remove_optional_float(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove float property {key}")))
    }

    fn remove_optional_bool(&mut self, key: &str) -> Result<Option<bool>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_bool())
                }
            })
            .transpose()
    }

    fn remove_bool(&mut self, key: &str) -> Result<bool, Error> {
        self.remove_optional_bool(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove float property {key}")))
    }
}
