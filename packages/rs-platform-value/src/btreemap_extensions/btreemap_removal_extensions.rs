use crate::{BinaryData, Bytes20, Bytes32, Error, Identifier, Value};
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
    fn remove_optional_identifier(&mut self, key: &str) -> Result<Option<Identifier>, Error>;
    fn remove_identifier(&mut self, key: &str) -> Result<Identifier, Error>;
    fn remove_binary_data(&mut self, key: &str) -> Result<BinaryData, Error>;
    fn remove_optional_binary_data(&mut self, key: &str) -> Result<Option<BinaryData>, Error>;
    fn remove_optional_bytes_32(&mut self, key: &str) -> Result<Option<Bytes32>, Error>;
    fn remove_bytes_32(&mut self, key: &str) -> Result<Bytes32, Error>;
    fn remove_optional_bytes_20(&mut self, key: &str) -> Result<Option<Bytes20>, Error>;
    fn remove_bytes_20(&mut self, key: &str) -> Result<Bytes20, Error>;
    fn remove_optional_hash256s(&mut self, key: &str) -> Result<Option<Vec<[u8; 32]>>, Error>;
    fn remove_hash256s(&mut self, key: &str) -> Result<Vec<[u8; 32]>, Error>;
    fn remove_identifiers(&mut self, key: &str) -> Result<Vec<Identifier>, Error>;
    fn remove_optional_identifiers(&mut self, key: &str) -> Result<Option<Vec<Identifier>>, Error>;
    fn remove_map_as_btree_map<K, V>(&mut self, key: &str) -> Result<BTreeMap<K, V>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
        V: TryFrom<Value, Error = Error>;
    fn remove_optional_map_as_btree_map<K, V>(
        &mut self,
        key: &str,
    ) -> Result<Option<BTreeMap<K, V>>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
        V: TryFrom<Value, Error = Error>;
}

pub trait BTreeValueRemoveTupleFromMapHelper {
    fn remove_tuple<K, V>(&mut self, key: &str) -> Result<(K, V), Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
        V: TryFrom<Value, Error = Error>;
    fn remove_optional_tuple<K, V>(&mut self, key: &str) -> Result<Option<(K, V)>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
        V: TryFrom<Value, Error = Error>;
}

impl BTreeValueRemoveFromMapHelper for BTreeMap<String, &Value> {
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

    fn remove_optional_bool(&mut self, key: &str) -> Result<Option<bool>, Error> {
        self.remove(key)
            .and_then(|v| if v.is_null() { None } else { Some(v.to_bool()) })
            .transpose()
    }

    fn remove_bool(&mut self, key: &str) -> Result<bool, Error> {
        self.remove_optional_bool(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove float property {key}")))
    }

    fn remove_optional_identifier(&mut self, key: &str) -> Result<Option<Identifier>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_identifier())
                }
            })
            .transpose()
    }

    fn remove_identifier(&mut self, key: &str) -> Result<Identifier, Error> {
        self.remove_optional_identifier(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove identifier property {key}"))
        })
    }

    fn remove_binary_data(&mut self, key: &str) -> Result<BinaryData, Error> {
        self.remove_optional_binary_data(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove binary data property {key}"))
        })
    }

    fn remove_optional_binary_data(&mut self, key: &str) -> Result<Option<BinaryData>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_binary_data())
                }
            })
            .transpose()
    }

    fn remove_optional_bytes_32(&mut self, key: &str) -> Result<Option<Bytes32>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_bytes_32())
                }
            })
            .transpose()
    }

    fn remove_bytes_32(&mut self, key: &str) -> Result<Bytes32, Error> {
        self.remove_optional_bytes_32(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove binary 32 bytes property {key}"))
        })
    }

    fn remove_optional_bytes_20(&mut self, key: &str) -> Result<Option<Bytes20>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_bytes_20())
                }
            })
            .transpose()
    }

    fn remove_bytes_20(&mut self, key: &str) -> Result<Bytes20, Error> {
        self.remove_optional_bytes_20(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove binary bytes 20 property {key}"))
        })
    }

    fn remove_optional_hash256s(&mut self, key: &str) -> Result<Option<Vec<[u8; 32]>>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else if let Value::Array(array) = v {
                    Some(
                        array
                            .iter()
                            .map(|item| item.clone().into_hash256())
                            .collect(),
                    )
                } else {
                    None
                }
            })
            .transpose()
    }

    fn remove_hash256s(&mut self, key: &str) -> Result<Vec<[u8; 32]>, Error> {
        self.remove_optional_hash256s(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove identifier property {key}"))
        })
    }

    fn remove_identifiers(&mut self, key: &str) -> Result<Vec<Identifier>, Error> {
        self.remove_optional_identifiers(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove identifier property {key}"))
        })
    }

    fn remove_optional_identifiers(&mut self, key: &str) -> Result<Option<Vec<Identifier>>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else if let Value::Array(array) = v {
                    Some(array.iter().map(|item| item.to_identifier()).collect())
                } else {
                    None
                }
            })
            .transpose()
    }

    fn remove_map_as_btree_map<K, V>(&mut self, key: &str) -> Result<BTreeMap<K, V>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
        V: TryFrom<Value, Error = Error>,
    {
        self.remove_optional_map_as_btree_map(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove map property {key}")))
    }

    fn remove_optional_map_as_btree_map<K, V>(
        &mut self,
        key: &str,
    ) -> Result<Option<BTreeMap<K, V>>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
        V: TryFrom<Value, Error = Error>,
    {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else if let Value::Map(map) = v {
                    Some(
                        map.iter()
                            .map(|(key, value)| {
                                Ok((key.clone().try_into()?, value.clone().try_into()?))
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            })
            .transpose()
    }
}

impl BTreeValueRemoveFromMapHelper for BTreeMap<String, Value> {
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

    fn remove_optional_identifier(&mut self, key: &str) -> Result<Option<Identifier>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_identifier())
                }
            })
            .transpose()
    }

    fn remove_identifier(&mut self, key: &str) -> Result<Identifier, Error> {
        self.remove_optional_identifier(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove identifier property {key}"))
        })
    }

    fn remove_binary_data(&mut self, key: &str) -> Result<BinaryData, Error> {
        self.remove_optional_binary_data(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove bytes property {key}")))
    }

    fn remove_optional_binary_data(&mut self, key: &str) -> Result<Option<BinaryData>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_binary_data())
                }
            })
            .transpose()
    }

    fn remove_optional_bytes_32(&mut self, key: &str) -> Result<Option<Bytes32>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_bytes_32())
                }
            })
            .transpose()
    }

    fn remove_bytes_32(&mut self, key: &str) -> Result<Bytes32, Error> {
        self.remove_optional_bytes_32(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove binary bytes 32 property {key}"))
        })
    }

    fn remove_optional_bytes_20(&mut self, key: &str) -> Result<Option<Bytes20>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.into_bytes_20())
                }
            })
            .transpose()
    }

    fn remove_bytes_20(&mut self, key: &str) -> Result<Bytes20, Error> {
        self.remove_optional_bytes_20(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove binary bytes 20 property {key}"))
        })
    }

    fn remove_optional_hash256s(&mut self, key: &str) -> Result<Option<Vec<[u8; 32]>>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else if let Value::Array(array) = v {
                    Some(array.into_iter().map(|item| item.into_hash256()).collect())
                } else {
                    None
                }
            })
            .transpose()
    }

    fn remove_hash256s(&mut self, key: &str) -> Result<Vec<[u8; 32]>, Error> {
        self.remove_optional_hash256s(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove identifier property {key}"))
        })
    }

    fn remove_identifiers(&mut self, key: &str) -> Result<Vec<Identifier>, Error> {
        self.remove_optional_identifiers(key)?.ok_or_else(|| {
            Error::StructureError(format!("unable to remove identifier property {key}"))
        })
    }

    fn remove_optional_identifiers(&mut self, key: &str) -> Result<Option<Vec<Identifier>>, Error> {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else if let Value::Array(array) = v {
                    Some(
                        array
                            .into_iter()
                            .map(|item| item.into_identifier())
                            .collect(),
                    )
                } else {
                    None
                }
            })
            .transpose()
    }

    fn remove_map_as_btree_map<K, V>(&mut self, key: &str) -> Result<BTreeMap<K, V>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
        V: TryFrom<Value, Error = Error>,
    {
        self.remove_optional_map_as_btree_map(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove map property {key}")))
    }

    fn remove_optional_map_as_btree_map<K, V>(
        &mut self,
        key: &str,
    ) -> Result<Option<BTreeMap<K, V>>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
        V: TryFrom<Value, Error = Error>,
    {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else if let Value::Map(map) = v {
                    Some(
                        map.into_iter()
                            .map(|(key, value)| Ok((key.try_into()?, value.try_into()?)))
                            .collect(),
                    )
                } else {
                    None
                }
            })
            .transpose()
    }
}

impl BTreeValueRemoveTupleFromMapHelper for BTreeMap<String, Value> {
    fn remove_tuple<K, V>(&mut self, key: &str) -> Result<(K, V), Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
        V: TryFrom<Value, Error = Error>,
    {
        self.remove_optional_tuple(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove tuple property {key}")))
    }

    fn remove_optional_tuple<K, V>(&mut self, key: &str) -> Result<Option<(K, V)>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
        V: TryFrom<Value, Error = Error>,
    {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else if let Value::Array(mut arr) = v {
                    if arr.len() == 2 {
                        let key_value = match arr.remove(0).try_into() {
                            Ok(key_value) => key_value,
                            Err(e) => return Some(Err(e)),
                        };
                        let value_value: V = match arr.remove(1).try_into() {
                            Ok(key_value) => key_value,
                            Err(e) => return Some(Err(e)),
                        };
                        Some(Ok((key_value, value_value)))
                    } else {
                        Some(Err(Error::StructureError(format!(
                            "Value for key {key} is not a tuple of length 2"
                        ))))
                    }
                } else {
                    Some(Err(Error::StructureError(format!(
                        "Value for key {key} is not an array"
                    ))))
                }
            })
            .transpose()
    }
}
