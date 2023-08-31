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
