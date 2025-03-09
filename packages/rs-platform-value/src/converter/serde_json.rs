use crate::value_map::ValueMap;
use crate::{Error, Value};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::{Map, Number};
use std::collections::BTreeMap;

impl Value {
    pub fn convert_from_serde_json_map<I, R>(map: I) -> R
    where
        I: IntoIterator<Item = (String, serde_json::Value)>,
        R: FromIterator<(String, Value)>,
    {
        map.into_iter()
            .map(|(key, serde_json_value)| (key, serde_json_value.into()))
            .collect()
    }

    pub fn try_into_validating_json(self) -> Result<serde_json::Value, Error> {
        Ok(match self {
            Value::U128(i) => {
                if i > u64::MAX as u128 {
                    return Err(Error::IntegerSizeError);
                }
                serde_json::Value::Number((i as u64).into())
            }
            Value::I128(i) => {
                if i > i64::MAX as i128 {
                    return Err(Error::IntegerSizeError);
                }
                if i < i64::MIN as i128 {
                    return Err(Error::IntegerSizeError);
                }
                serde_json::Value::Number((i as i64).into())
            }
            Value::U64(i) => serde_json::Value::Number(i.into()),
            Value::I64(i) => serde_json::Value::Number(i.into()),
            Value::U32(i) => serde_json::Value::Number(i.into()),
            Value::I32(i) => serde_json::Value::Number(i.into()),
            Value::U16(i) => serde_json::Value::Number(i.into()),
            Value::I16(i) => serde_json::Value::Number(i.into()),
            Value::U8(i) => serde_json::Value::Number(i.into()),
            Value::I8(i) => serde_json::Value::Number(i.into()),
            Value::Float(float) => {
                serde_json::Value::Number(Number::from_f64(float).unwrap_or(0.into()))
            }
            Value::Text(string) => serde_json::Value::String(string),
            Value::Bool(value) => serde_json::Value::Bool(value),
            Value::Null => serde_json::Value::Null,
            Value::Array(array) => serde_json::Value::Array(
                array
                    .into_iter()
                    .map(|value| value.try_into_validating_json())
                    .collect::<Result<Vec<serde_json::Value>, Error>>()?,
            ),
            Value::Map(map) => serde_json::Value::Object(
                map.into_iter()
                    .map(|(k, v)| {
                        let string = k.into_text()?;
                        Ok((string, v.try_into_validating_json()?))
                    })
                    .collect::<Result<Map<String, serde_json::Value>, Error>>()?,
            ),
            Value::Identifier(bytes) => {
                // In order to be able to validate using JSON schema it needs to be in byte form
                serde_json::Value::Array(
                    bytes
                        .into_iter()
                        .map(|a| serde_json::Value::Number(a.into()))
                        .collect(),
                )
            }
            Value::Bytes(bytes) => serde_json::Value::Array(
                bytes
                    .into_iter()
                    .map(|byte| serde_json::Value::Number(byte.into()))
                    .collect(),
            ),
            Value::Bytes20(bytes) => serde_json::Value::Array(
                bytes
                    .into_iter()
                    .map(|byte| serde_json::Value::Number(byte.into()))
                    .collect(),
            ),
            Value::Bytes32(bytes) => serde_json::Value::Array(
                bytes
                    .into_iter()
                    .map(|byte| serde_json::Value::Number(byte.into()))
                    .collect(),
            ),
            Value::Bytes36(bytes) => serde_json::Value::Array(
                bytes
                    .into_iter()
                    .map(|byte| serde_json::Value::Number(byte.into()))
                    .collect(),
            ),
            Value::EnumU8(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumU8 to JSONValue".to_string(),
                ))
            }
            Value::EnumString(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumString to JSONValue".to_string(),
                ))
            }
        })
    }

    pub fn try_into_validating_btree_map_json(
        self,
    ) -> Result<BTreeMap<String, serde_json::Value>, Error> {
        self.into_btree_string_map()?
            .into_iter()
            .map(|(key, value)| Ok((key, value.try_into_validating_json()?)))
            .collect()
    }

    pub fn try_to_validating_json(&self) -> Result<serde_json::Value, Error> {
        Ok(match self {
            Value::U128(i) => {
                if *i > u64::MAX as u128 {
                    return Err(Error::IntegerSizeError);
                }
                serde_json::Value::Number((*i as u64).into())
            }
            Value::I128(i) => {
                if *i > i64::MAX as i128 {
                    return Err(Error::IntegerSizeError);
                }
                if *i < i64::MIN as i128 {
                    return Err(Error::IntegerSizeError);
                }
                serde_json::Value::Number((*i as i64).into())
            }
            Value::U64(i) => serde_json::Value::Number((*i).into()),
            Value::I64(i) => serde_json::Value::Number((*i).into()),
            Value::U32(i) => serde_json::Value::Number((*i).into()),
            Value::I32(i) => serde_json::Value::Number((*i).into()),
            Value::U16(i) => serde_json::Value::Number((*i).into()),
            Value::I16(i) => serde_json::Value::Number((*i).into()),
            Value::U8(i) => serde_json::Value::Number((*i).into()),
            Value::I8(i) => serde_json::Value::Number((*i).into()),
            Value::Float(float) => {
                serde_json::Value::Number(Number::from_f64(*float).unwrap_or(0.into()))
            }
            Value::Text(string) => serde_json::Value::String(string.clone()),
            Value::Bool(value) => serde_json::Value::Bool(*value),
            Value::Null => serde_json::Value::Null,
            Value::Array(array) => serde_json::Value::Array(
                array
                    .iter()
                    .map(|value| value.try_to_validating_json())
                    .collect::<Result<Vec<serde_json::Value>, Error>>()?,
            ),
            Value::Map(map) => serde_json::Value::Object(
                map.iter()
                    .map(|(k, v)| {
                        let string = k.to_text()?;
                        Ok((string, v.try_to_validating_json()?))
                    })
                    .collect::<Result<Map<String, serde_json::Value>, Error>>()?,
            ),
            Value::Identifier(bytes) => {
                // In order to be able to validate using JSON schema it needs to be in byte form
                serde_json::Value::Array(
                    bytes
                        .iter()
                        .map(|a| serde_json::Value::Number((*a).into()))
                        .collect(),
                )
            }
            Value::Bytes(bytes) => serde_json::Value::Array(
                bytes
                    .iter()
                    .map(|byte| serde_json::Value::Number((*byte).into()))
                    .collect(),
            ),
            Value::Bytes20(bytes) => serde_json::Value::Array(
                bytes
                    .iter()
                    .map(|byte| serde_json::Value::Number((*byte).into()))
                    .collect(),
            ),
            Value::Bytes32(bytes) => serde_json::Value::Array(
                bytes
                    .iter()
                    .map(|byte| serde_json::Value::Number((*byte).into()))
                    .collect(),
            ),
            Value::Bytes36(bytes) => serde_json::Value::Array(
                bytes
                    .iter()
                    .map(|byte| serde_json::Value::Number((*byte).into()))
                    .collect(),
            ),
            Value::EnumU8(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumU8 to JSONValue".to_string(),
                ))
            }
            Value::EnumString(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumString to JSONValue".to_string(),
                ))
            }
        })
    }
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(value),
            serde_json::Value::Number(number) => {
                if let Some(value) = number.as_u64() {
                    return Self::U64(value);
                } else if let Some(value) = number.as_i64() {
                    return Self::I64(value);
                } else if let Some(value) = number.as_f64() {
                    return Self::Float(value);
                }
                unreachable!("this shouldn't be reachable")
            }
            serde_json::Value::String(string) => Self::Text(string),
            serde_json::Value::Array(array) => {
                let u8_max = u8::MAX as u64;
                //todo: hacky solution, to fix
                let len = array.len();
                if len >= 10
                    && array.iter().all(|v| {
                        let Some(int) = v.as_u64() else {
                            return false;
                        };
                        int.le(&u8_max)
                    })
                {
                    //this is an array of bytes
                    Self::Bytes(
                        array
                            .into_iter()
                            .map(|v| v.as_u64().unwrap() as u8)
                            .collect(),
                    )
                } else {
                    Self::Array(array.into_iter().map(|v| v.into()).collect())
                }
            }
            serde_json::Value::Object(map) => {
                Self::Map(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
            }
        }
    }
}

impl From<&serde_json::Value> for Value {
    fn from(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(*value),
            serde_json::Value::Number(number) => {
                if let Some(value) = number.as_u64() {
                    return Self::U64(value);
                } else if let Some(value) = number.as_i64() {
                    return Self::I64(value);
                } else if let Some(value) = number.as_f64() {
                    return Self::Float(value);
                }
                unreachable!("this shouldn't be reachable")
            }
            serde_json::Value::String(string) => Self::Text(string.clone()),
            serde_json::Value::Array(array) => {
                let u8_max = u8::MAX as u64;
                //todo: hacky solution, to fix
                let len = array.len();
                if len >= 10
                    && array.iter().all(|v| {
                        let Some(int) = v.as_u64() else {
                            return false;
                        };
                        int.le(&u8_max)
                    })
                {
                    //this is an array of bytes
                    Self::Bytes(array.iter().map(|v| v.as_u64().unwrap() as u8).collect())
                } else {
                    Self::Array(array.iter().map(|v| v.into()).collect())
                }
            }
            serde_json::Value::Object(map) => Self::Map(
                map.into_iter()
                    .map(|(k, v)| (k.clone().into(), v.into()))
                    .collect(),
            ),
        }
    }
}

impl From<Box<serde_json::Value>> for Box<Value> {
    fn from(value: Box<serde_json::Value>) -> Self {
        value.into()
    }
}

impl TryInto<serde_json::Value> for Value {
    type Error = Error;

    fn try_into(self) -> Result<serde_json::Value, Self::Error> {
        Ok(match self {
            Value::U128(i) => serde_json::Value::Number((i as u64).into()),
            Value::I128(i) => serde_json::Value::Number((i as i64).into()),
            Value::U64(i) => serde_json::Value::Number(i.into()),
            Value::I64(i) => serde_json::Value::Number(i.into()),
            Value::U32(i) => serde_json::Value::Number(i.into()),
            Value::I32(i) => serde_json::Value::Number(i.into()),
            Value::U16(i) => serde_json::Value::Number(i.into()),
            Value::I16(i) => serde_json::Value::Number(i.into()),
            Value::U8(i) => serde_json::Value::Number(i.into()),
            Value::I8(i) => serde_json::Value::Number(i.into()),
            Value::Bytes(bytes) => {
                serde_json::Value::String(BASE64_STANDARD.encode(bytes.as_slice()))
            }
            Value::Bytes20(bytes) => {
                serde_json::Value::String(BASE64_STANDARD.encode(bytes.as_slice()))
            }
            Value::Bytes32(bytes) => {
                serde_json::Value::String(BASE64_STANDARD.encode(bytes.as_slice()))
            }
            Value::Bytes36(bytes) => {
                serde_json::Value::String(BASE64_STANDARD.encode(bytes.as_slice()))
            }
            Value::Float(float) => {
                serde_json::Value::Number(Number::from_f64(float).unwrap_or(0.into()))
            }
            Value::Text(string) => serde_json::Value::String(string),
            Value::Bool(value) => serde_json::Value::Bool(value),
            Value::Null => serde_json::Value::Null,
            Value::Array(array) => serde_json::Value::Array(
                array
                    .into_iter()
                    .map(|value| value.try_into())
                    .collect::<Result<Vec<serde_json::Value>, Error>>()?,
            ),
            Value::Map(map) => serde_json::Value::Object(
                map.into_iter()
                    .map(|(k, v)| {
                        let string = k.into_text()?;
                        Ok((string, v.try_into()?))
                    })
                    .collect::<Result<Map<String, serde_json::Value>, Error>>()?,
            ),
            Value::Identifier(bytes) => {
                serde_json::Value::String(bs58::encode(bytes.as_slice()).into_string())
            }
            Value::EnumU8(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumU8 to JSONValue".to_string(),
                ))
            }
            Value::EnumString(_) => {
                return Err(Error::Unsupported(
                    "No support for conversion of EnumString to JSONValue".to_string(),
                ))
            }
        })
    }
}

pub trait BTreeValueJsonConverter {
    fn into_json_value(self) -> Result<serde_json::Value, Error>;
    fn into_validating_json_value(self) -> Result<serde_json::Value, Error>;
    fn to_json_value(&self) -> Result<serde_json::Value, Error>;
    fn to_validating_json_value(&self) -> Result<serde_json::Value, Error>;
    fn from_json_value(value: serde_json::Value) -> Result<Self, Error>
    where
        Self: Sized;
}

impl BTreeValueJsonConverter for BTreeMap<String, Value> {
    fn into_json_value(self) -> Result<serde_json::Value, Error> {
        Ok(serde_json::Value::Object(
            self.into_iter()
                .map(|(key, value)| Ok((key, value.try_into()?)))
                .collect::<Result<Map<String, serde_json::Value>, Error>>()?,
        ))
    }

    fn into_validating_json_value(self) -> Result<serde_json::Value, Error> {
        Ok(serde_json::Value::Object(
            self.into_iter()
                .map(|(key, value)| Ok((key, value.try_into_validating_json()?)))
                .collect::<Result<Map<String, serde_json::Value>, Error>>()?,
        ))
    }

    fn to_json_value(&self) -> Result<serde_json::Value, Error> {
        Ok(serde_json::Value::Object(
            self.iter()
                .map(|(key, value)| Ok((key.clone(), value.clone().try_into()?)))
                .collect::<Result<Map<String, serde_json::Value>, Error>>()?,
        ))
    }

    fn to_validating_json_value(&self) -> Result<serde_json::Value, Error> {
        Ok(serde_json::Value::Object(
            self.iter()
                .map(|(key, value)| Ok((key.to_owned(), value.try_to_validating_json()?)))
                .collect::<Result<Map<String, serde_json::Value>, Error>>()?,
        ))
    }

    fn from_json_value(value: serde_json::Value) -> Result<Self, Error> {
        let platform_value: Value = value.into();
        platform_value.into_btree_string_map()
    }
}

impl From<BTreeMap<String, serde_json::Value>> for Value {
    fn from(value: BTreeMap<String, serde_json::Value>) -> Self {
        let map: ValueMap = value
            .into_iter()
            .map(|(key, json_value)| {
                let value: Value = json_value.into();
                (Value::Text(key), value)
            })
            .collect();
        Value::Map(map)
    }
}

impl From<&BTreeMap<String, serde_json::Value>> for Value {
    fn from(value: &BTreeMap<String, serde_json::Value>) -> Self {
        let map: ValueMap = value
            .iter()
            .map(|(key, json_value)| {
                let value: Value = json_value.into();
                (Value::Text(key.clone()), value)
            })
            .collect();
        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use crate::Value;
    use serde_json::json;

    #[test]
    fn test_json_array() {
        let json = json!({
          "type": 5,
          "protocolVersion": 1,
          "revision": 0,
          "signature": "HxtcTSpRdACokorvpx/f4ezM40e0WtgW2GUvjiwNkHPwKDppkIoS2cirhqpZURlhDuYdu+E0KllbHNlYghcK9Bg=",
          "signaturePublicKeyId": 1,
          "addPublicKeys": [
            {
              "id": 0,
              "purpose": 0,
              "securityLevel": 0,
              "type": 0,
              "data": "Aya0WP8EhKQ6Dq+51sAnqdPah664X9CUciVJYAfvfTnX",
              "readOnly": false,
              "signature": "HxtcTSpRdACokorvpx/f4ezM40e0WtgW2GUvjiwNkHPwKDppkIoS2cirhqpZURlhDuYdu+E0KllbHNlYghcK9Bg="
            }
          ],
          "disablePublicKeys": [ 0 ],
          "identityId": "62DHhTfZV3NvUbXUha1mavLqSEy2GaWYja2qeTYNUhk"
        });

        let value: Value = json.into();
        let array = value
            .get_optional_array_slice("addPublicKeys")
            .expect("expected to get array slice")
            .unwrap();
        assert_eq!(array.len(), 1);
        assert!(array.first().unwrap().is_map());
        let array = value
            .get_optional_array_slice("disablePublicKeys")
            .expect("expected to get array slice")
            .unwrap();
        assert_eq!(array.len(), 1);
    }
}
