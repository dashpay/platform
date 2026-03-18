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

    fn remove_map_as_btree_map_keep_values_as_platform_value<K, V>(
        &mut self,
        key: &str,
    ) -> Result<BTreeMap<K, Value>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord;

    fn remove_optional_map_as_btree_map_keep_values_as_platform_value<K>(
        &mut self,
        key: &str,
    ) -> Result<Option<BTreeMap<K, Value>>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord;
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

    fn remove_map_as_btree_map_keep_values_as_platform_value<K, V>(
        &mut self,
        key: &str,
    ) -> Result<BTreeMap<K, Value>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
    {
        self.remove_optional_map_as_btree_map_keep_values_as_platform_value(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove map property {key}")))
    }

    fn remove_optional_map_as_btree_map_keep_values_as_platform_value<K>(
        &mut self,
        key: &str,
    ) -> Result<Option<BTreeMap<K, Value>>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
    {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else if let Value::Map(map) = v {
                    Some(
                        map.iter()
                            .map(|(key, value)| Ok((key.clone().try_into()?, value.clone())))
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

    fn remove_map_as_btree_map_keep_values_as_platform_value<K, V>(
        &mut self,
        key: &str,
    ) -> Result<BTreeMap<K, Value>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
    {
        self.remove_optional_map_as_btree_map_keep_values_as_platform_value(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove map property {key}")))
    }

    fn remove_optional_map_as_btree_map_keep_values_as_platform_value<K>(
        &mut self,
        key: &str,
    ) -> Result<Option<BTreeMap<K, Value>>, Error>
    where
        K: TryFrom<Value, Error = Error> + Ord,
    {
        self.remove(key)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else if let Value::Map(map) = v {
                    Some(
                        map.into_iter()
                            .map(|(key, value)| Ok((key.try_into()?, value)))
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
                        // After removing index 0 above, the second element is now at index 0
                        let value_value: V = match arr.remove(0).try_into() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryData, Bytes20, Bytes32, Identifier, Value};
    use std::collections::BTreeMap;

    // -----------------------------------------------------------------------
    // Helper functions
    // -----------------------------------------------------------------------

    fn owned_map_with(entries: Vec<(&str, Value)>) -> BTreeMap<String, Value> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    fn ref_map_from(map: &BTreeMap<String, Value>) -> BTreeMap<String, &Value> {
        map.iter().map(|(k, v)| (k.clone(), v)).collect()
    }

    // -----------------------------------------------------------------------
    // Tests for BTreeMap<String, Value> (owned impl)
    // -----------------------------------------------------------------------

    mod owned_map {
        use super::*;

        // -- String tests --

        #[test]
        fn remove_string_success() {
            let mut map = owned_map_with(vec![("name", Value::Text("alice".to_string()))]);
            let result = map.remove_string("name").unwrap();
            assert_eq!(result, "alice");
            assert!(map.is_empty());
        }

        #[test]
        fn remove_string_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_string("name").is_err());
        }

        #[test]
        fn remove_optional_string_returns_some() {
            let mut map = owned_map_with(vec![("name", Value::Text("bob".to_string()))]);
            let result = map.remove_optional_string("name").unwrap();
            assert_eq!(result, Some("bob".to_string()));
        }

        #[test]
        fn remove_optional_string_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_string("name").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_string_returns_none_for_null() {
            let mut map = owned_map_with(vec![("name", Value::Null)]);
            let result = map.remove_optional_string("name").unwrap();
            assert_eq!(result, None);
        }

        // -- Float tests --

        #[test]
        fn remove_float_success() {
            let mut map = owned_map_with(vec![("score", Value::Float(1.23))]);
            let result = map.remove_float("score").unwrap();
            assert!((result - 1.23).abs() < f64::EPSILON);
        }

        #[test]
        fn remove_float_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_float("score").is_err());
        }

        #[test]
        fn remove_optional_float_returns_some() {
            let mut map = owned_map_with(vec![("score", Value::Float(2.72))]);
            let result = map.remove_optional_float("score").unwrap();
            assert_eq!(result, Some(2.72));
        }

        #[test]
        fn remove_optional_float_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_float("score").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_float_returns_none_for_null() {
            let mut map = owned_map_with(vec![("score", Value::Null)]);
            let result = map.remove_optional_float("score").unwrap();
            assert_eq!(result, None);
        }

        // -- Integer tests --

        #[test]
        fn remove_integer_u64_success() {
            let mut map = owned_map_with(vec![("age", Value::U64(42))]);
            let result: u64 = map.remove_integer("age").unwrap();
            assert_eq!(result, 42);
        }

        #[test]
        fn remove_integer_i32_success() {
            let mut map = owned_map_with(vec![("temp", Value::I32(-10))]);
            let result: i32 = map.remove_integer("temp").unwrap();
            assert_eq!(result, -10);
        }

        #[test]
        fn remove_integer_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result: Result<u64, Error> = map.remove_integer("age");
            assert!(result.is_err());
        }

        #[test]
        fn remove_optional_integer_returns_some() {
            let mut map = owned_map_with(vec![("count", Value::U32(7))]);
            let result: Option<u32> = map.remove_optional_integer("count").unwrap();
            assert_eq!(result, Some(7));
        }

        #[test]
        fn remove_optional_integer_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result: Option<u32> = map.remove_optional_integer("count").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_integer_returns_none_for_null() {
            let mut map = owned_map_with(vec![("count", Value::Null)]);
            let result: Option<u32> = map.remove_optional_integer("count").unwrap();
            assert_eq!(result, None);
        }

        // -- Bool tests --

        #[test]
        fn remove_bool_success() {
            let mut map = owned_map_with(vec![("active", Value::Bool(true))]);
            let result = map.remove_bool("active").unwrap();
            assert!(result);
        }

        #[test]
        fn remove_bool_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_bool("active").is_err());
        }

        #[test]
        fn remove_optional_bool_returns_some() {
            let mut map = owned_map_with(vec![("flag", Value::Bool(false))]);
            let result = map.remove_optional_bool("flag").unwrap();
            assert_eq!(result, Some(false));
        }

        #[test]
        fn remove_optional_bool_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_bool("flag").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_bool_returns_none_for_null() {
            let mut map = owned_map_with(vec![("flag", Value::Null)]);
            let result = map.remove_optional_bool("flag").unwrap();
            assert_eq!(result, None);
        }

        // -- Hash256 bytes tests --

        #[test]
        fn remove_hash256_bytes_success() {
            let hash = [1u8; 32];
            let mut map = owned_map_with(vec![("hash", Value::Bytes32(hash))]);
            let result = map.remove_hash256_bytes("hash").unwrap();
            assert_eq!(result, hash);
        }

        #[test]
        fn remove_hash256_bytes_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_hash256_bytes("hash").is_err());
        }

        #[test]
        fn remove_optional_hash256_bytes_returns_some() {
            let hash = [2u8; 32];
            let mut map = owned_map_with(vec![("hash", Value::Bytes32(hash))]);
            let result = map.remove_optional_hash256_bytes("hash").unwrap();
            assert_eq!(result, Some(hash));
        }

        #[test]
        fn remove_optional_hash256_bytes_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_hash256_bytes("hash").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_hash256_bytes_returns_none_for_null() {
            let mut map = owned_map_with(vec![("hash", Value::Null)]);
            let result = map.remove_optional_hash256_bytes("hash").unwrap();
            assert_eq!(result, None);
        }

        // -- Bytes tests --

        #[test]
        fn remove_bytes_success() {
            let bytes = vec![1, 2, 3, 4];
            let mut map = owned_map_with(vec![("data", Value::Bytes(bytes.clone()))]);
            let result = map.remove_bytes("data").unwrap();
            assert_eq!(result, bytes);
        }

        #[test]
        fn remove_bytes_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_bytes("data").is_err());
        }

        #[test]
        fn remove_optional_bytes_returns_some() {
            let bytes = vec![5, 6, 7];
            let mut map = owned_map_with(vec![("data", Value::Bytes(bytes.clone()))]);
            let result = map.remove_optional_bytes("data").unwrap();
            assert_eq!(result, Some(bytes));
        }

        #[test]
        fn remove_optional_bytes_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_bytes("data").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_bytes_returns_none_for_null() {
            let mut map = owned_map_with(vec![("data", Value::Null)]);
            let result = map.remove_optional_bytes("data").unwrap();
            assert_eq!(result, None);
        }

        // -- Identifier tests --

        #[test]
        fn remove_identifier_success() {
            let id_bytes = [3u8; 32];
            let mut map = owned_map_with(vec![("id", Value::Identifier(id_bytes))]);
            let result = map.remove_identifier("id").unwrap();
            assert_eq!(result, Identifier::from(id_bytes));
        }

        #[test]
        fn remove_identifier_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_identifier("id").is_err());
        }

        #[test]
        fn remove_optional_identifier_returns_some() {
            let id_bytes = [4u8; 32];
            let mut map = owned_map_with(vec![("id", Value::Identifier(id_bytes))]);
            let result = map.remove_optional_identifier("id").unwrap();
            assert_eq!(result, Some(Identifier::from(id_bytes)));
        }

        #[test]
        fn remove_optional_identifier_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_identifier("id").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_identifier_returns_none_for_null() {
            let mut map = owned_map_with(vec![("id", Value::Null)]);
            let result = map.remove_optional_identifier("id").unwrap();
            assert_eq!(result, None);
        }

        // -- BinaryData tests --

        #[test]
        fn remove_binary_data_success() {
            let bytes = vec![10, 20, 30];
            let mut map = owned_map_with(vec![("bin", Value::Bytes(bytes.clone()))]);
            let result = map.remove_binary_data("bin").unwrap();
            assert_eq!(result, BinaryData(bytes));
        }

        #[test]
        fn remove_binary_data_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_binary_data("bin").is_err());
        }

        #[test]
        fn remove_optional_binary_data_returns_some() {
            let bytes = vec![40, 50];
            let mut map = owned_map_with(vec![("bin", Value::Bytes(bytes.clone()))]);
            let result = map.remove_optional_binary_data("bin").unwrap();
            assert_eq!(result, Some(BinaryData(bytes)));
        }

        #[test]
        fn remove_optional_binary_data_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_binary_data("bin").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_binary_data_returns_none_for_null() {
            let mut map = owned_map_with(vec![("bin", Value::Null)]);
            let result = map.remove_optional_binary_data("bin").unwrap();
            assert_eq!(result, None);
        }

        // -- Bytes32 tests --

        #[test]
        fn remove_bytes_32_success() {
            let b32 = [5u8; 32];
            let mut map = owned_map_with(vec![("b32", Value::Bytes32(b32))]);
            let result = map.remove_bytes_32("b32").unwrap();
            assert_eq!(result, Bytes32(b32));
        }

        #[test]
        fn remove_bytes_32_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_bytes_32("b32").is_err());
        }

        #[test]
        fn remove_optional_bytes_32_returns_some() {
            let b32 = [6u8; 32];
            let mut map = owned_map_with(vec![("b32", Value::Bytes32(b32))]);
            let result = map.remove_optional_bytes_32("b32").unwrap();
            assert_eq!(result, Some(Bytes32(b32)));
        }

        #[test]
        fn remove_optional_bytes_32_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_bytes_32("b32").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_bytes_32_returns_none_for_null() {
            let mut map = owned_map_with(vec![("b32", Value::Null)]);
            let result = map.remove_optional_bytes_32("b32").unwrap();
            assert_eq!(result, None);
        }

        // -- Bytes20 tests --

        #[test]
        fn remove_bytes_20_success() {
            let b20 = [7u8; 20];
            let mut map = owned_map_with(vec![("b20", Value::Bytes20(b20))]);
            let result = map.remove_bytes_20("b20").unwrap();
            assert_eq!(result, Bytes20(b20));
        }

        #[test]
        fn remove_bytes_20_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_bytes_20("b20").is_err());
        }

        #[test]
        fn remove_optional_bytes_20_returns_some() {
            let b20 = [8u8; 20];
            let mut map = owned_map_with(vec![("b20", Value::Bytes20(b20))]);
            let result = map.remove_optional_bytes_20("b20").unwrap();
            assert_eq!(result, Some(Bytes20(b20)));
        }

        #[test]
        fn remove_optional_bytes_20_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_bytes_20("b20").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_bytes_20_returns_none_for_null() {
            let mut map = owned_map_with(vec![("b20", Value::Null)]);
            let result = map.remove_optional_bytes_20("b20").unwrap();
            assert_eq!(result, None);
        }

        // -- Hash256s (array of hashes) tests --

        #[test]
        fn remove_hash256s_success() {
            let h1 = [1u8; 32];
            let h2 = [2u8; 32];
            let mut map = owned_map_with(vec![(
                "hashes",
                Value::Array(vec![Value::Bytes32(h1), Value::Bytes32(h2)]),
            )]);
            let result = map.remove_hash256s("hashes").unwrap();
            assert_eq!(result, vec![h1, h2]);
        }

        #[test]
        fn remove_hash256s_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_hash256s("hashes").is_err());
        }

        #[test]
        fn remove_optional_hash256s_returns_some() {
            let h1 = [3u8; 32];
            let mut map = owned_map_with(vec![("hashes", Value::Array(vec![Value::Bytes32(h1)]))]);
            let result = map.remove_optional_hash256s("hashes").unwrap();
            assert_eq!(result, Some(vec![h1]));
        }

        #[test]
        fn remove_optional_hash256s_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_hash256s("hashes").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_hash256s_returns_none_for_null() {
            let mut map = owned_map_with(vec![("hashes", Value::Null)]);
            let result = map.remove_optional_hash256s("hashes").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_hash256s_returns_none_for_non_array() {
            let mut map = owned_map_with(vec![("hashes", Value::Text("not_array".to_string()))]);
            let result = map.remove_optional_hash256s("hashes").unwrap();
            assert_eq!(result, None);
        }

        // -- Identifiers (array) tests --

        #[test]
        fn remove_identifiers_success() {
            let id1 = [10u8; 32];
            let id2 = [20u8; 32];
            let mut map = owned_map_with(vec![(
                "ids",
                Value::Array(vec![Value::Identifier(id1), Value::Identifier(id2)]),
            )]);
            let result = map.remove_identifiers("ids").unwrap();
            assert_eq!(result, vec![Identifier::from(id1), Identifier::from(id2)]);
        }

        #[test]
        fn remove_identifiers_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            assert!(map.remove_identifiers("ids").is_err());
        }

        #[test]
        fn remove_optional_identifiers_returns_some() {
            let id = [30u8; 32];
            let mut map = owned_map_with(vec![("ids", Value::Array(vec![Value::Identifier(id)]))]);
            let result = map.remove_optional_identifiers("ids").unwrap();
            assert_eq!(result, Some(vec![Identifier::from(id)]));
        }

        #[test]
        fn remove_optional_identifiers_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result = map.remove_optional_identifiers("ids").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_identifiers_returns_none_for_null() {
            let mut map = owned_map_with(vec![("ids", Value::Null)]);
            let result = map.remove_optional_identifiers("ids").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_identifiers_returns_none_for_non_array() {
            let mut map = owned_map_with(vec![("ids", Value::U64(999))]);
            let result = map.remove_optional_identifiers("ids").unwrap();
            assert_eq!(result, None);
        }

        // -- Map as BTreeMap tests --

        #[test]
        fn remove_map_as_btree_map_success() {
            let inner_map = vec![
                (Value::Text("a".to_string()), Value::U64(1)),
                (Value::Text("b".to_string()), Value::U64(2)),
            ];
            let mut map = owned_map_with(vec![("m", Value::Map(inner_map))]);
            let result: BTreeMap<String, u64> = map.remove_map_as_btree_map("m").unwrap();
            assert_eq!(result.get("a"), Some(&1u64));
            assert_eq!(result.get("b"), Some(&2u64));
        }

        #[test]
        fn remove_map_as_btree_map_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result: Result<BTreeMap<String, String>, Error> = map.remove_map_as_btree_map("m");
            assert!(result.is_err());
        }

        #[test]
        fn remove_optional_map_as_btree_map_returns_some() {
            let inner_map = vec![(Value::Text("x".to_string()), Value::Text("y".to_string()))];
            let mut map = owned_map_with(vec![("m", Value::Map(inner_map))]);
            let result: Option<BTreeMap<String, String>> =
                map.remove_optional_map_as_btree_map("m").unwrap();
            assert!(result.is_some());
            let inner = result.unwrap();
            assert_eq!(inner.get("x"), Some(&"y".to_string()));
        }

        #[test]
        fn remove_optional_map_as_btree_map_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result: Option<BTreeMap<String, String>> =
                map.remove_optional_map_as_btree_map("m").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_map_as_btree_map_returns_none_for_null() {
            let mut map = owned_map_with(vec![("m", Value::Null)]);
            let result: Option<BTreeMap<String, String>> =
                map.remove_optional_map_as_btree_map("m").unwrap();
            assert_eq!(result, None);
        }

        // -- Map keep values as platform value tests --

        #[test]
        fn remove_map_as_btree_map_keep_values_success() {
            let inner_map = vec![
                (Value::Text("k1".to_string()), Value::Bool(true)),
                (Value::Text("k2".to_string()), Value::Float(1.5)),
            ];
            let mut map = owned_map_with(vec![("m", Value::Map(inner_map))]);
            let result: BTreeMap<String, Value> = map
                .remove_map_as_btree_map_keep_values_as_platform_value::<String, Value>("m")
                .unwrap();
            assert_eq!(result.get("k1"), Some(&Value::Bool(true)));
            assert_eq!(result.get("k2"), Some(&Value::Float(1.5)));
        }

        #[test]
        fn remove_map_as_btree_map_keep_values_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result: Result<BTreeMap<String, Value>, Error> =
                map.remove_map_as_btree_map_keep_values_as_platform_value::<String, Value>("m");
            assert!(result.is_err());
        }

        #[test]
        fn remove_optional_map_keep_values_returns_none_for_null() {
            let mut map = owned_map_with(vec![("m", Value::Null)]);
            let result: Option<BTreeMap<String, Value>> = map
                .remove_optional_map_as_btree_map_keep_values_as_platform_value("m")
                .unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_map_keep_values_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result: Option<BTreeMap<String, Value>> = map
                .remove_optional_map_as_btree_map_keep_values_as_platform_value("m")
                .unwrap();
            assert_eq!(result, None);
        }

        // -- Removal actually removes the key from the map --

        #[test]
        fn removal_removes_key_from_map() {
            let mut map = owned_map_with(vec![
                ("a", Value::Text("hello".to_string())),
                ("b", Value::U64(42)),
            ]);
            assert_eq!(map.len(), 2);
            let _ = map.remove_string("a");
            assert_eq!(map.len(), 1);
            assert!(!map.contains_key("a"));
            assert!(map.contains_key("b"));
        }

        // -- Integer type edge cases --

        #[test]
        fn remove_integer_u8() {
            let mut map = owned_map_with(vec![("x", Value::U8(255))]);
            let result: u8 = map.remove_integer("x").unwrap();
            assert_eq!(result, 255);
        }

        #[test]
        fn remove_integer_i16() {
            let mut map = owned_map_with(vec![("x", Value::I16(-100))]);
            let result: i16 = map.remove_integer("x").unwrap();
            assert_eq!(result, -100);
        }

        #[test]
        fn remove_integer_u128() {
            let mut map = owned_map_with(vec![("x", Value::U128(u128::MAX))]);
            let result: u128 = map.remove_integer("x").unwrap();
            assert_eq!(result, u128::MAX);
        }

        #[test]
        fn remove_integer_i128() {
            let mut map = owned_map_with(vec![("x", Value::I128(i128::MIN))]);
            let result: i128 = map.remove_integer("x").unwrap();
            assert_eq!(result, i128::MIN);
        }

        // -- Tuple tests --

        #[test]
        fn remove_tuple_success() {
            let mut map = owned_map_with(vec![(
                "pair",
                Value::Array(vec![
                    Value::Text("key".to_string()),
                    Value::Text("value".to_string()),
                ]),
            )]);
            let result: (String, String) = map.remove_tuple("pair").unwrap();
            assert_eq!(result, ("key".to_string(), "value".to_string()));
        }

        #[test]
        fn remove_tuple_missing_key_errors() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result: Result<(String, String), Error> = map.remove_tuple("pair");
            assert!(result.is_err());
        }

        #[test]
        fn remove_optional_tuple_returns_some() {
            let mut map = owned_map_with(vec![(
                "pair",
                Value::Array(vec![
                    Value::Text("a".to_string()),
                    Value::Text("b".to_string()),
                ]),
            )]);
            let result: Option<(String, String)> = map.remove_optional_tuple("pair").unwrap();
            assert_eq!(result, Some(("a".to_string(), "b".to_string())));
        }

        #[test]
        fn remove_optional_tuple_returns_none_for_missing() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            let result: Option<(String, String)> = map.remove_optional_tuple("pair").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_tuple_returns_none_for_null() {
            let mut map = owned_map_with(vec![("pair", Value::Null)]);
            let result: Option<(String, String)> = map.remove_optional_tuple("pair").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_tuple_wrong_length_errors() {
            let mut map = owned_map_with(vec![(
                "pair",
                Value::Array(vec![Value::Text("only_one".to_string())]),
            )]);
            let result: Result<Option<(String, String)>, Error> = map.remove_optional_tuple("pair");
            assert!(result.is_err());
        }

        #[test]
        fn remove_optional_tuple_non_array_errors() {
            let mut map = owned_map_with(vec![("pair", Value::Text("not_an_array".to_string()))]);
            let result: Result<Option<(String, String)>, Error> = map.remove_optional_tuple("pair");
            assert!(result.is_err());
        }

        #[test]
        fn remove_optional_tuple_three_elements_errors() {
            let mut map = owned_map_with(vec![(
                "pair",
                Value::Array(vec![
                    Value::Text("a".to_string()),
                    Value::Text("b".to_string()),
                    Value::Text("c".to_string()),
                ]),
            )]);
            let result: Result<Option<(String, String)>, Error> = map.remove_optional_tuple("pair");
            assert!(result.is_err());
        }

        #[test]
        fn remove_optional_tuple_empty_array_errors() {
            let mut map = owned_map_with(vec![("pair", Value::Array(vec![]))]);
            let result: Result<Option<(String, String)>, Error> = map.remove_optional_tuple("pair");
            assert!(result.is_err());
        }
    }

    // -----------------------------------------------------------------------
    // Tests for BTreeMap<String, &Value> (ref impl)
    // -----------------------------------------------------------------------

    mod ref_map {
        use super::*;

        // -- String tests --

        #[test]
        fn remove_string_success() {
            let source = owned_map_with(vec![("name", Value::Text("alice".to_string()))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_string("name").unwrap();
            assert_eq!(result, "alice");
        }

        #[test]
        fn remove_string_missing_key_errors() {
            let mut map: BTreeMap<String, &Value> = BTreeMap::new();
            assert!(map.remove_string("name").is_err());
        }

        #[test]
        fn remove_optional_string_returns_some() {
            let source = owned_map_with(vec![("name", Value::Text("bob".to_string()))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_string("name").unwrap();
            assert_eq!(result, Some("bob".to_string()));
        }

        #[test]
        fn remove_optional_string_returns_none_for_missing() {
            let mut map: BTreeMap<String, &Value> = BTreeMap::new();
            let result = map.remove_optional_string("name").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_string_returns_none_for_null() {
            let source = owned_map_with(vec![("name", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_string("name").unwrap();
            assert_eq!(result, None);
        }

        // -- Float tests --

        #[test]
        fn remove_float_success() {
            let source = owned_map_with(vec![("f", Value::Float(2.5))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_float("f").unwrap();
            assert!((result - 2.5).abs() < f64::EPSILON);
        }

        #[test]
        fn remove_float_missing_key_errors() {
            let mut map: BTreeMap<String, &Value> = BTreeMap::new();
            assert!(map.remove_float("f").is_err());
        }

        #[test]
        fn remove_optional_float_returns_none_for_null() {
            let source = owned_map_with(vec![("f", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_float("f").unwrap();
            assert_eq!(result, None);
        }

        // -- Integer tests --

        #[test]
        fn remove_integer_success() {
            let source = owned_map_with(vec![("num", Value::U64(99))]);
            let mut map = ref_map_from(&source);
            let result: u64 = map.remove_integer("num").unwrap();
            assert_eq!(result, 99);
        }

        #[test]
        fn remove_integer_missing_key_errors() {
            let mut map: BTreeMap<String, &Value> = BTreeMap::new();
            let result: Result<u64, Error> = map.remove_integer("num");
            assert!(result.is_err());
        }

        #[test]
        fn remove_optional_integer_returns_none_for_null() {
            let source = owned_map_with(vec![("num", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result: Option<u64> = map.remove_optional_integer("num").unwrap();
            assert_eq!(result, None);
        }

        // -- Bool tests --

        #[test]
        fn remove_bool_success() {
            let source = owned_map_with(vec![("flag", Value::Bool(true))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_bool("flag").unwrap();
            assert!(result);
        }

        #[test]
        fn remove_optional_bool_returns_none_for_null() {
            let source = owned_map_with(vec![("flag", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_bool("flag").unwrap();
            assert_eq!(result, None);
        }

        // -- Hash256 tests --

        #[test]
        fn remove_hash256_bytes_success() {
            let hash = [9u8; 32];
            let source = owned_map_with(vec![("h", Value::Bytes32(hash))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_hash256_bytes("h").unwrap();
            assert_eq!(result, hash);
        }

        #[test]
        fn remove_optional_hash256_bytes_returns_none_for_null() {
            let source = owned_map_with(vec![("h", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_hash256_bytes("h").unwrap();
            assert_eq!(result, None);
        }

        // -- Bytes tests --

        #[test]
        fn remove_bytes_success() {
            let bytes = vec![1, 2, 3];
            let source = owned_map_with(vec![("b", Value::Bytes(bytes.clone()))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_bytes("b").unwrap();
            assert_eq!(result, bytes);
        }

        #[test]
        fn remove_optional_bytes_returns_none_for_null() {
            let source = owned_map_with(vec![("b", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_bytes("b").unwrap();
            assert_eq!(result, None);
        }

        // -- Identifier tests --

        #[test]
        fn remove_identifier_success() {
            let id_bytes = [11u8; 32];
            let source = owned_map_with(vec![("id", Value::Identifier(id_bytes))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_identifier("id").unwrap();
            assert_eq!(result, Identifier::from(id_bytes));
        }

        #[test]
        fn remove_optional_identifier_returns_none_for_null() {
            let source = owned_map_with(vec![("id", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_identifier("id").unwrap();
            assert_eq!(result, None);
        }

        // -- BinaryData tests --

        #[test]
        fn remove_binary_data_success() {
            let bytes = vec![100, 200];
            let source = owned_map_with(vec![("bd", Value::Bytes(bytes.clone()))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_binary_data("bd").unwrap();
            assert_eq!(result, BinaryData(bytes));
        }

        #[test]
        fn remove_optional_binary_data_returns_none_for_null() {
            let source = owned_map_with(vec![("bd", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_binary_data("bd").unwrap();
            assert_eq!(result, None);
        }

        // -- Bytes32 tests --

        #[test]
        fn remove_bytes_32_success() {
            let b32 = [12u8; 32];
            let source = owned_map_with(vec![("b32", Value::Bytes32(b32))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_bytes_32("b32").unwrap();
            assert_eq!(result, Bytes32(b32));
        }

        #[test]
        fn remove_optional_bytes_32_returns_none_for_null() {
            let source = owned_map_with(vec![("b32", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_bytes_32("b32").unwrap();
            assert_eq!(result, None);
        }

        // -- Bytes20 tests --

        #[test]
        fn remove_bytes_20_success() {
            let b20 = [13u8; 20];
            let source = owned_map_with(vec![("b20", Value::Bytes20(b20))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_bytes_20("b20").unwrap();
            assert_eq!(result, Bytes20(b20));
        }

        #[test]
        fn remove_optional_bytes_20_returns_none_for_null() {
            let source = owned_map_with(vec![("b20", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_bytes_20("b20").unwrap();
            assert_eq!(result, None);
        }

        // -- Hash256s (array of hashes) tests --

        #[test]
        fn remove_hash256s_success() {
            let h1 = [14u8; 32];
            let h2 = [15u8; 32];
            let source = owned_map_with(vec![(
                "hashes",
                Value::Array(vec![Value::Bytes32(h1), Value::Bytes32(h2)]),
            )]);
            let mut map = ref_map_from(&source);
            let result = map.remove_hash256s("hashes").unwrap();
            assert_eq!(result, vec![h1, h2]);
        }

        #[test]
        fn remove_optional_hash256s_returns_none_for_null() {
            let source = owned_map_with(vec![("hashes", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_hash256s("hashes").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_hash256s_returns_none_for_non_array() {
            let source = owned_map_with(vec![("hashes", Value::U64(42))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_hash256s("hashes").unwrap();
            assert_eq!(result, None);
        }

        // -- Identifiers (array) tests --

        #[test]
        fn remove_identifiers_success() {
            let id1 = [16u8; 32];
            let source = owned_map_with(vec![("ids", Value::Array(vec![Value::Identifier(id1)]))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_identifiers("ids").unwrap();
            assert_eq!(result, vec![Identifier::from(id1)]);
        }

        #[test]
        fn remove_optional_identifiers_returns_none_for_null() {
            let source = owned_map_with(vec![("ids", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_identifiers("ids").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_identifiers_returns_none_for_non_array() {
            let source = owned_map_with(vec![("ids", Value::Bool(false))]);
            let mut map = ref_map_from(&source);
            let result = map.remove_optional_identifiers("ids").unwrap();
            assert_eq!(result, None);
        }

        // -- Map as BTreeMap tests --

        #[test]
        fn remove_optional_map_as_btree_map_returns_none_for_null() {
            let source = owned_map_with(vec![("m", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result: Option<BTreeMap<String, String>> =
                map.remove_optional_map_as_btree_map("m").unwrap();
            assert_eq!(result, None);
        }

        #[test]
        fn remove_optional_map_as_btree_map_returns_none_for_missing() {
            let mut map: BTreeMap<String, &Value> = BTreeMap::new();
            let result: Option<BTreeMap<String, String>> =
                map.remove_optional_map_as_btree_map("m").unwrap();
            assert_eq!(result, None);
        }

        // -- Map keep values tests --

        #[test]
        fn remove_optional_map_keep_values_returns_none_for_null() {
            let source = owned_map_with(vec![("m", Value::Null)]);
            let mut map = ref_map_from(&source);
            let result: Option<BTreeMap<String, Value>> = map
                .remove_optional_map_as_btree_map_keep_values_as_platform_value("m")
                .unwrap();
            assert_eq!(result, None);
        }

        // -- Ref map removal semantics --

        #[test]
        fn ref_removal_removes_key_from_map() {
            let source = owned_map_with(vec![("a", Value::Bool(true)), ("b", Value::Bool(false))]);
            let mut map = ref_map_from(&source);
            assert_eq!(map.len(), 2);
            let _ = map.remove_bool("a");
            assert_eq!(map.len(), 1);
            assert!(!map.contains_key("a"));
        }
    }
}
