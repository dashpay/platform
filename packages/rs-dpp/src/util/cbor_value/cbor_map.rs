use ciborium::value::Value as CborValue;
use std::convert::TryFrom;
use std::{collections::BTreeMap, convert::TryInto};

use crate::ProtocolError;

use super::value_to_bytes;

pub trait CborBTreeMapHelper {
    fn get_identifier(&self, key: &str) -> Result<[u8; 32], ProtocolError>;
    fn get_string(&self, key: &str) -> Result<String, ProtocolError>;
    fn get_integer<T: TryFrom<i128>>(&self, key: &str) -> Result<T, ProtocolError>;
}

pub trait CborMapExtension {
    fn as_u16(&self, key: &str, error_message: &str) -> Result<u16, ProtocolError>;
    fn as_u8(&self, key: &str, error_message: &str) -> Result<u8, ProtocolError>;
    fn as_bool(&self, key: &str, error_message: &str) -> Result<bool, ProtocolError>;
    fn as_bytes(&self, key: &str, error_message: &str) -> Result<Vec<u8>, ProtocolError>;
    fn as_string(&self, key: &str, error_message: &str) -> Result<String, ProtocolError>;
    fn as_u64(&self, key: &str, error_message: &str) -> Result<u64, ProtocolError>;
}

impl CborBTreeMapHelper for BTreeMap<String, CborValue> {
    fn get_identifier(&self, key: &str) -> Result<[u8; 32], ProtocolError> {
        map_values_to_bytes(self, key)?
            .ok_or_else(|| ProtocolError::DecodingError(format!("unable to get {key}")))?
            .try_into()
            .map_err(|_| ProtocolError::DecodingError(format!("{key} must be 32 bytes")))
    }

    fn get_string(&self, key: &str) -> Result<String, ProtocolError> {
        Ok(self
            .get(key)
            .ok_or_else(|| ProtocolError::DecodingError(format!("unable to get {key}")))?
            .as_text()
            .ok_or_else(|| ProtocolError::DecodingError(format!("expect {key} to be a string")))?
            .to_string())
    }

    fn get_integer<T: TryFrom<i128>>(&self, key: &str) -> Result<T, ProtocolError> {
        i128::from(
            self.get(key)
                .ok_or_else(|| {
                    ProtocolError::DecodingError(format!("unable to get property {key}"))
                })?
                .as_integer()
                .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be an integer")))?,
        )
        .try_into()
        .map_err(|_| ProtocolError::DecodingError(format!("{key} is out of required bounds")))
    }
}

pub fn map_values_to_bytes(
    document: &BTreeMap<String, CborValue>,
    key: &str,
) -> Result<Option<Vec<u8>>, ProtocolError> {
    let value = document.get(key);
    if let Some(value) = value {
        value_to_bytes(value)
    } else {
        Ok(None)
    }
}
