use crate::value_map::ValueMapHelper;
use crate::{Error, Value};
use std::collections::BTreeMap;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use std::iter::Peekable;
use std::vec::IntoIter;

#[derive(Debug, Clone, Copy)]
pub enum IntegerReplacementType {
    U128,
    I128,
    U64,
    I64,
    U32,
    I32,
    U16,
    I16,
    U8,
    I8,
}

impl IntegerReplacementType {
    pub fn replace_for_value(&self, value: Value) -> Result<Value, Error> {
        Ok(match self {
            IntegerReplacementType::U128 => Value::U128(value.try_into()?),
            IntegerReplacementType::I128 => Value::I128(value.try_into()?),
            IntegerReplacementType::U64 => Value::U64(value.try_into()?),
            IntegerReplacementType::I64 => Value::I64(value.try_into()?),
            IntegerReplacementType::U32 => Value::U32(value.try_into()?),
            IntegerReplacementType::I32 => Value::I32(value.try_into()?),
            IntegerReplacementType::U16 => Value::U16(value.try_into()?),
            IntegerReplacementType::I16 => Value::I16(value.try_into()?),
            IntegerReplacementType::U8 => Value::U8(value.try_into()?),
            IntegerReplacementType::I8 => Value::I8(value.try_into()?),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReplacementType {
    Identifier,
    BinaryBytes,
    TextBase58,
    TextBase64,
}

impl ReplacementType {
    pub fn replace_for_bytes(&self, bytes: Vec<u8>) -> Result<Value, Error> {
        match self {
            ReplacementType::Identifier => {
                Ok(Value::Identifier(bytes.try_into().map_err(|_| {
                    Error::ByteLengthNot32BytesError(String::from(
                        "Trying to replace into an identifier, but not 32 bytes long",
                    ))
                })?))
            }
            ReplacementType::BinaryBytes => Ok(Value::Bytes(bytes)),
            ReplacementType::TextBase58 => Ok(Value::Text(bs58::encode(bytes).into_string())),
            ReplacementType::TextBase64 => Ok(Value::Text(BASE64_STANDARD.encode(bytes))),
        }
    }

    pub fn replace_for_bytes_20(&self, bytes: [u8; 20]) -> Result<Value, Error> {
        match self {
            ReplacementType::BinaryBytes => Ok(Value::Bytes20(bytes)),
            ReplacementType::TextBase58 => Ok(Value::Text(bs58::encode(bytes).into_string())),
            ReplacementType::TextBase64 => Ok(Value::Text(BASE64_STANDARD.encode(bytes))),
            _ => Err(Error::ByteLengthNot36BytesError(
                "trying to replace 36 bytes into an identifier".to_string(),
            )),
        }
    }

    pub fn replace_for_bytes_32(&self, bytes: [u8; 32]) -> Result<Value, Error> {
        match self {
            ReplacementType::Identifier => Ok(Value::Identifier(bytes)),
            ReplacementType::BinaryBytes => Ok(Value::Bytes32(bytes)),
            ReplacementType::TextBase58 => Ok(Value::Text(bs58::encode(bytes).into_string())),
            ReplacementType::TextBase64 => Ok(Value::Text(BASE64_STANDARD.encode(bytes))),
        }
    }

    pub fn replace_for_bytes_36(&self, bytes: [u8; 36]) -> Result<Value, Error> {
        match self {
            ReplacementType::BinaryBytes => Ok(Value::Bytes36(bytes)),
            ReplacementType::TextBase58 => Ok(Value::Text(bs58::encode(bytes).into_string())),
            ReplacementType::TextBase64 => Ok(Value::Text(BASE64_STANDARD.encode(bytes))),
            _ => Err(Error::ByteLengthNot36BytesError(
                "trying to replace 36 bytes into an identifier".to_string(),
            )),
        }
    }

    pub fn replace_consume_value(&self, value: Value) -> Result<Value, Error> {
        let bytes = value.into_identifier_bytes()?;
        self.replace_for_bytes(bytes)
    }

    pub fn replace_value_in_place(&self, value: &mut Value) -> Result<(), Error> {
        let bytes = value.take().into_identifier_bytes()?;
        *value = self.replace_for_bytes(bytes)?;
        Ok(())
    }
}

pub trait BTreeValueMapReplacementPathHelper {
    fn replace_at_path(
        &mut self,
        path: &str,
        replacement_type: ReplacementType,
    ) -> Result<(), Error>;
    fn replace_at_paths<'a, I: IntoIterator<Item = &'a String>>(
        &mut self,
        paths: I,
        replacement_type: ReplacementType,
    ) -> Result<(), Error>;
}

fn replace_down(
    mut current_values: Vec<&mut Value>,
    mut split: Peekable<IntoIter<&str>>,
    replacement_type: ReplacementType,
) -> Result<(), Error> {
    if let Some(path_component) = split.next() {
        let next_values = current_values
            .iter_mut()
            .map(|current_value| {
                if current_value.is_map() {
                    let map = current_value.as_map_mut_ref()?;
                    let Some(new_value) = map.get_optional_key_mut(path_component) else {
                        return Ok(None);
                    };
                    if split.peek().is_none() {
                        match new_value {
                            Value::Bytes20(bytes) => {
                                *new_value = replacement_type.replace_for_bytes_20(*bytes)?;
                            }
                            Value::Bytes32(bytes) => {
                                *new_value = replacement_type.replace_for_bytes_32(*bytes)?;
                            }
                            Value::Bytes36(bytes) => {
                                *new_value = replacement_type.replace_for_bytes_36(*bytes)?;
                            }
                            _ => {
                                let bytes = match replacement_type {
                                    ReplacementType::Identifier | ReplacementType::TextBase58 => {
                                        new_value.to_identifier_bytes()
                                    }
                                    ReplacementType::BinaryBytes | ReplacementType::TextBase64 => {
                                        new_value.to_binary_bytes()
                                    }
                                }?;
                                *new_value = replacement_type.replace_for_bytes(bytes)?;
                            }
                        }
                        Ok(None)
                    } else {
                        Ok(Some(vec![new_value]))
                    }
                } else if current_value.is_array() {
                    // if it's an array we apply to all members
                    let array = current_value.to_array_mut()?.iter_mut().collect();
                    Ok(Some(array))
                } else {
                    Err(Error::PathError("path was not an array or map".to_string()))
                }
            })
            .collect::<Result<Vec<_>, Error>>()?
            .into_iter()
            .flatten()
            .flatten()
            .collect();
        replace_down(next_values, split, replacement_type)
    } else {
        Ok(())
    }
}

impl BTreeValueMapReplacementPathHelper for BTreeMap<String, Value> {
    fn replace_at_path(
        &mut self,
        path: &str,
        replacement_type: ReplacementType,
    ) -> Result<(), Error> {
        let mut split: Vec<_> = path.split('.').collect();
        let first = split.first();
        let Some(first_path_component) = first else {
            return Err(Error::PathError("path was empty".to_string()));
        };
        let Some(current_value) = self.get_mut(first_path_component.to_owned()) else {
            return Ok(());
        };
        if split.len() == 1 {
            match current_value {
                Value::Bytes20(bytes) => {
                    *current_value = replacement_type.replace_for_bytes_20(*bytes)?;
                }
                Value::Bytes32(bytes) => {
                    *current_value = replacement_type.replace_for_bytes_32(*bytes)?;
                }
                Value::Bytes36(bytes) => {
                    *current_value = replacement_type.replace_for_bytes_36(*bytes)?;
                }
                _ => {
                    let bytes = match replacement_type {
                        ReplacementType::Identifier | ReplacementType::TextBase58 => {
                            current_value.to_identifier_bytes()
                        }
                        ReplacementType::BinaryBytes | ReplacementType::TextBase64 => {
                            current_value.to_binary_bytes()
                        }
                    }?;
                    *current_value = replacement_type.replace_for_bytes(bytes)?;
                }
            }
            Ok(())
        } else {
            split.remove(0);
            let current_values = vec![current_value];
            //todo: make this non recursive
            replace_down(
                current_values,
                split.into_iter().peekable(),
                replacement_type,
            )
        }
    }

    fn replace_at_paths<'a, I: IntoIterator<Item = &'a String>>(
        &mut self,
        paths: I,
        replacement_type: ReplacementType,
    ) -> Result<(), Error> {
        paths
            .into_iter()
            .try_for_each(|path| self.replace_at_path(path.as_str(), replacement_type))
    }
}
