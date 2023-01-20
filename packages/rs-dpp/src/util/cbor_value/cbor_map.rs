use ciborium::value::Value as CborValue;
use std::convert::TryFrom;
use std::{collections::BTreeMap, convert::TryInto};
use std::borrow::Borrow;
use futures::FutureExt;

use crate::ProtocolError;
use crate::util::cbor_value::value_to_hash;

use super::value_to_bytes;

pub trait CborBTreeMapHelper {
    fn get_optional_identifier(&self, key: &str) -> Result<Option<[u8; 32]>, ProtocolError>;
    fn get_identifier(&self, key: &str) -> Result<[u8; 32], ProtocolError>;
    fn get_string(&self, key: &str) -> Result<String, ProtocolError>;
    fn get_optional_integer<T: TryFrom<i128>>(&self, key: &str) -> Result<Option<T>, ProtocolError>;
    fn get_integer<T: TryFrom<i128>>(&self, key: &str) -> Result<T, ProtocolError>;
    fn get_optional_bool(&self, key: &str) -> Result<Option<bool>, ProtocolError>;
    fn get_bool(&self, key: &str) -> Result<bool, ProtocolError>;
    fn get_optional_inner_value_array<I: IntoIterator<Item = CborValue>>(&self, key: &str) -> Result<Option<I>, ProtocolError>;
    fn get_inner_value_array<I: IntoIterator<Item = CborValue>>(&self, key: &str) -> Result<I, ProtocolError>;
    fn get_optional_inner_string_array<I: IntoIterator<Item = String>>(&self, key: &str) -> Result<Option<I>, ProtocolError>;
    fn get_inner_string_array<I: IntoIterator<Item = String>>(&self, key: &str) -> Result<I, ProtocolError>;
    fn get_optional_inner_string_value_map<I: IntoIterator<Item = (String, CborValue)>>(&self, key: &str) -> Result<Option<I>, ProtocolError>;
    fn get_inner_string_value_map<I: IntoIterator<Item = (String, CborValue)>>(&self, key: &str) -> Result<I, ProtocolError>;
}

pub trait CborMapExtension {
    fn as_u16(&self, key: &str, error_message: &str) -> Result<u16, ProtocolError>;
    fn as_u8(&self, key: &str, error_message: &str) -> Result<u8, ProtocolError>;
    fn as_bool(&self, key: &str, error_message: &str) -> Result<bool, ProtocolError>;
    fn as_bytes(&self, key: &str, error_message: &str) -> Result<Vec<u8>, ProtocolError>;
    fn as_string(&self, key: &str, error_message: &str) -> Result<String, ProtocolError>;
    fn as_u64(&self, key: &str, error_message: &str) -> Result<u64, ProtocolError>;
}

impl <V>CborBTreeMapHelper for BTreeMap<String, V>
where V: Borrow<CborValue>
{
    fn get_optional_identifier(&self, key: &str) -> Result<Option<[u8; 32]>, ProtocolError> {
        self.get(key).map(|i|value_to_hash(i.borrow())).transpose()
    }

    fn get_identifier(&self, key: &str) -> Result<[u8; 32], ProtocolError> {
        self.get_optional_identifier(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get property {key}"))
        })
    }

    fn get_string(&self, key: &str) -> Result<String, ProtocolError> {
        Ok(self
            .get(key)
            .ok_or_else(|| ProtocolError::DecodingError(format!("unable to get {key}")))?
            .borrow().as_text()
            .ok_or_else(|| ProtocolError::DecodingError(format!("expect {key} to be a string")))?
            .to_string())
    }

    fn get_optional_integer<T: TryFrom<i128>>(&self, key: &str) -> Result<Option<T>, ProtocolError> {
        self.get(key).map(|v| {
            i128::from(v.borrow().as_integer()
                .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be an integer")))?)
                .try_into()
                .map_err(|_| ProtocolError::DecodingError(format!("{key} is out of required bounds")))
        }).transpose()
    }

    fn get_integer<T: TryFrom<i128>>(&self, key: &str) -> Result<T, ProtocolError> {
        self.get_optional_integer(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get property {key}"))
        })
    }

    fn get_optional_bool(&self, key: &str) -> Result<Option<bool>, ProtocolError> {
        self.get(key).map(|v| {
            v.borrow().as_bool()
                .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a bool")))
        }).transpose()
    }

    fn get_bool(&self, key: &str) -> Result<bool, ProtocolError> {
        self.get_optional_bool(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get property {key}"))
        })
    }

    fn get_optional_inner_value_array<I: IntoIterator<Item = CborValue>>(&self, key: &str) -> Result<Option<I>, ProtocolError> {
        self.get(key).map(|v| {
            v.borrow().as_array()
                .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a bool")))
        }).transpose()
    }

    fn get_inner_value_array<I: IntoIterator<Item = CborValue>>(&self, key: &str) -> Result<I, ProtocolError> {
        self.get_optional_inner_value_array(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get property {key}"))
        })
    }

    fn get_optional_inner_string_array<I: IntoIterator<Item = String>>(&self, key: &str) -> Result<Option<I>, ProtocolError> {
        self.get(key).map(|v| {
            v.borrow().as_array().map(|inner| {
                inner.iter().map(|(v)| {
                    let Some(str) = v.as_text() else {
                        return Err(ProtocolError::DecodingError(format!("{k} must be an string")))
                    };
                    Ok(str)
                }).collect::<Result<I,ProtocolError>>()
            }).transpose()?
                .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a bool")))
        }).transpose()
    }

    fn get_inner_string_array<I: IntoIterator<Item = String>>(&self, key: &str) -> Result<I, ProtocolError> {
        self.get_optional_inner_string_array(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get property {key}"))
        })
    }

    fn get_optional_inner_string_value_map<I: IntoIterator<Item = (String, CborValue)>>(&self, key: &str) -> Result<Option<I>, ProtocolError> {
        self.get(key).map(|v| {
            v.borrow().as_map().map(|inner| {
                inner.iter().map(|(k, v)| {
                    let Some(str) = k.as_text() else {
                        return Err(ProtocolError::DecodingError(format!("{k} must be an string")))
                    };
                    Ok((str, v))
                }).collect::<Result<I,ProtocolError>>()
            }).transpose()?
                .ok_or_else(|| ProtocolError::DecodingError(format!("{key} must be a bool")))
        }).transpose()
    }

    fn get_inner_string_value_map<I: IntoIterator<Item = (String, CborValue)>>(&self, key: &str) -> Result<I, ProtocolError> {
        self.get_optional_inner_string_value_map(key)?.ok_or_else(|| {
            ProtocolError::DecodingError(format!("unable to get property {key}"))
        })
    }
}

