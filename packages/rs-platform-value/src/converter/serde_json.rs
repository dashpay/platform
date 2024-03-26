use crate::value_map::ValueMap;
use crate::{Error, Value};
use serde_json::{Map, Number, Value as JsonValue};
use std::collections::BTreeMap;

impl Value {
    pub fn convert_from_serde_json_map<I, R>(map: I) -> R
    where
        I: IntoIterator<Item = (String, JsonValue)>,
        R: FromIterator<(String, Value)>,
    {
        map.into_iter()
            .map(|(key, serde_json_value)| (key, serde_json_value.into()))
            .collect()
    }

    pub fn try_into_validating_json(self) -> Result<JsonValue, Error> {
        Ok(match self {
            Value::U128(i) => {
                if i > u64::MAX as u128 {
                    return Err(Error::IntegerSizeError);
                }
                JsonValue::Number((i as u64).into())
            }
            Value::I128(i) => {
                if i > i64::MAX as i128 {
                    return Err(Error::IntegerSizeError);
                }
                if i < i64::MIN as i128 {
                    return Err(Error::IntegerSizeError);
                }
                JsonValue::Number((i as i64).into())
            }
            Value::U64(i) => JsonValue::Number(i.into()),
            Value::I64(i) => JsonValue::Number(i.into()),
            Value::U32(i) => JsonValue::Number(i.into()),
            Value::I32(i) => JsonValue::Number(i.into()),
            Value::U16(i) => JsonValue::Number(i.into()),
            Value::I16(i) => JsonValue::Number(i.into()),
            Value::U8(i) => JsonValue::Number(i.into()),
            Value::I8(i) => JsonValue::Number(i.into()),
            Value::Float(float) => JsonValue::Number(Number::from_f64(float).unwrap_or(0.into())),
            Value::Text(string) => JsonValue::String(string),
            Value::Bool(value) => JsonValue::Bool(value),
            Value::Null => JsonValue::Null,
            Value::Array(array) => JsonValue::Array(
                array
                    .into_iter()
                    .map(|value| value.try_into_validating_json())
                    .collect::<Result<Vec<JsonValue>, Error>>()?,
            ),
            Value::Map(map) => JsonValue::Object(
                map.into_iter()
                    .map(|(k, v)| {
                        let string = k.into_text()?;
                        Ok((string, v.try_into_validating_json()?))
                    })
                    .collect::<Result<Map<String, JsonValue>, Error>>()?,
            ),
            Value::Identifier(bytes) => {
                // In order to be able to validate using JSON schema it needs to be in byte form
                JsonValue::Array(
                    bytes
                        .into_iter()
                        .map(|a| JsonValue::Number(a.into()))
                        .collect(),
                )
            }
            Value::Bytes(bytes) => JsonValue::Array(
                bytes
                    .into_iter()
                    .map(|byte| JsonValue::Number(byte.into()))
                    .collect(),
            ),
            Value::Bytes20(bytes) => JsonValue::Array(
                bytes
                    .into_iter()
                    .map(|byte| JsonValue::Number(byte.into()))
                    .collect(),
            ),
            Value::Bytes32(bytes) => JsonValue::Array(
                bytes
                    .into_iter()
                    .map(|byte| JsonValue::Number(byte.into()))
                    .collect(),
            ),
            Value::Bytes36(bytes) => JsonValue::Array(
                bytes
                    .into_iter()
                    .map(|byte| JsonValue::Number(byte.into()))
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

    pub fn try_into_validating_btree_map_json(self) -> Result<BTreeMap<String, JsonValue>, Error> {
        self.into_btree_string_map()?
            .into_iter()
            .map(|(key, value)| Ok((key, value.try_into_validating_json()?)))
            .collect()
    }

    pub fn try_to_validating_json(&self) -> Result<JsonValue, Error> {
        Ok(match self {
            Value::U128(i) => {
                if *i > u64::MAX as u128 {
                    return Err(Error::IntegerSizeError);
                }
                JsonValue::Number((*i as u64).into())
            }
            Value::I128(i) => {
                if *i > i64::MAX as i128 {
                    return Err(Error::IntegerSizeError);
                }
                if *i < i64::MIN as i128 {
                    return Err(Error::IntegerSizeError);
                }
                JsonValue::Number((*i as i64).into())
            }
            Value::U64(i) => JsonValue::Number((*i).into()),
            Value::I64(i) => JsonValue::Number((*i).into()),
            Value::U32(i) => JsonValue::Number((*i).into()),
            Value::I32(i) => JsonValue::Number((*i).into()),
            Value::U16(i) => JsonValue::Number((*i).into()),
            Value::I16(i) => JsonValue::Number((*i).into()),
            Value::U8(i) => JsonValue::Number((*i).into()),
            Value::I8(i) => JsonValue::Number((*i).into()),
            Value::Float(float) => JsonValue::Number(Number::from_f64(*float).unwrap_or(0.into())),
            Value::Text(string) => JsonValue::String(string.clone()),
            Value::Bool(value) => JsonValue::Bool(*value),
            Value::Null => JsonValue::Null,
            Value::Array(array) => JsonValue::Array(
                array
                    .iter()
                    .map(|value| value.try_to_validating_json())
                    .collect::<Result<Vec<JsonValue>, Error>>()?,
            ),
            Value::Map(map) => JsonValue::Object(
                map.iter()
                    .map(|(k, v)| {
                        let string = k.to_text()?;
                        Ok((string, v.try_to_validating_json()?))
                    })
                    .collect::<Result<Map<String, JsonValue>, Error>>()?,
            ),
            Value::Identifier(bytes) => {
                // In order to be able to validate using JSON schema it needs to be in byte form
                JsonValue::Array(
                    bytes
                        .iter()
                        .map(|a| JsonValue::Number((*a).into()))
                        .collect(),
                )
            }
            Value::Bytes(bytes) => JsonValue::Array(
                bytes
                    .iter()
                    .map(|byte| JsonValue::Number((*byte).into()))
                    .collect(),
            ),
            Value::Bytes20(bytes) => JsonValue::Array(
                bytes
                    .iter()
                    .map(|byte| JsonValue::Number((*byte).into()))
                    .collect(),
            ),
            Value::Bytes32(bytes) => JsonValue::Array(
                bytes
                    .iter()
                    .map(|byte| JsonValue::Number((*byte).into()))
                    .collect(),
            ),
            Value::Bytes36(bytes) => JsonValue::Array(
                bytes
                    .iter()
                    .map(|byte| JsonValue::Number((*byte).into()))
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

impl From<JsonValue> for Value {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(value) => Self::Bool(value),
            JsonValue::Number(number) => {
                if let Some(value) = number.as_u64() {
                    return Self::U64(value);
                } else if let Some(value) = number.as_i64() {
                    return Self::I64(value);
                } else if let Some(value) = number.as_f64() {
                    return Self::Float(value);
                }
                unreachable!("this shouldn't be reachable")
            }
            JsonValue::String(string) => Self::Text(string),
            JsonValue::Array(array) => {
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
            JsonValue::Object(map) => {
                Self::Map(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
            }
        }
    }
}

impl From<&JsonValue> for Value {
    fn from(value: &JsonValue) -> Self {
        match value {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(value) => Self::Bool(*value),
            JsonValue::Number(number) => {
                if let Some(value) = number.as_u64() {
                    return Self::U64(value);
                } else if let Some(value) = number.as_i64() {
                    return Self::I64(value);
                } else if let Some(value) = number.as_f64() {
                    return Self::Float(value);
                }
                unreachable!("this shouldn't be reachable")
            }
            JsonValue::String(string) => Self::Text(string.clone()),
            JsonValue::Array(array) => {
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
            JsonValue::Object(map) => Self::Map(
                map.into_iter()
                    .map(|(k, v)| (k.clone().into(), v.into()))
                    .collect(),
            ),
        }
    }
}

impl From<Box<JsonValue>> for Box<Value> {
    fn from(value: Box<JsonValue>) -> Self {
        value.into()
    }
}

impl TryInto<JsonValue> for Value {
    type Error = Error;

    fn try_into(self) -> Result<JsonValue, Self::Error> {
        Ok(match self {
            Value::U128(i) => JsonValue::Number((i as u64).into()),
            Value::I128(i) => JsonValue::Number((i as i64).into()),
            Value::U64(i) => JsonValue::Number(i.into()),
            Value::I64(i) => JsonValue::Number(i.into()),
            Value::U32(i) => JsonValue::Number(i.into()),
            Value::I32(i) => JsonValue::Number(i.into()),
            Value::U16(i) => JsonValue::Number(i.into()),
            Value::I16(i) => JsonValue::Number(i.into()),
            Value::U8(i) => JsonValue::Number(i.into()),
            Value::I8(i) => JsonValue::Number(i.into()),
            Value::Bytes(bytes) => JsonValue::String(base64::encode(bytes.as_slice())),
            Value::Bytes20(bytes) => JsonValue::String(base64::encode(bytes.as_slice())),
            Value::Bytes32(bytes) => JsonValue::String(base64::encode(bytes.as_slice())),
            Value::Bytes36(bytes) => JsonValue::String(base64::encode(bytes.as_slice())),
            Value::Float(float) => JsonValue::Number(Number::from_f64(float).unwrap_or(0.into())),
            Value::Text(string) => JsonValue::String(string),
            Value::Bool(value) => JsonValue::Bool(value),
            Value::Null => JsonValue::Null,
            Value::Array(array) => JsonValue::Array(
                array
                    .into_iter()
                    .map(|value| value.try_into())
                    .collect::<Result<Vec<JsonValue>, Error>>()?,
            ),
            Value::Map(map) => JsonValue::Object(
                map.into_iter()
                    .map(|(k, v)| {
                        let string = k.into_text()?;
                        Ok((string, v.try_into()?))
                    })
                    .collect::<Result<Map<String, JsonValue>, Error>>()?,
            ),
            Value::Identifier(bytes) => {
                JsonValue::String(bs58::encode(bytes.as_slice()).into_string())
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
    fn into_json_value(self) -> Result<JsonValue, Error>;
    fn into_validating_json_value(self) -> Result<JsonValue, Error>;
    fn to_json_value(&self) -> Result<JsonValue, Error>;
    fn to_validating_json_value(&self) -> Result<JsonValue, Error>;
    fn from_json_value(value: JsonValue) -> Result<Self, Error>
    where
        Self: Sized;
}

impl BTreeValueJsonConverter for BTreeMap<String, Value> {
    fn into_json_value(self) -> Result<JsonValue, Error> {
        Ok(JsonValue::Object(
            self.into_iter()
                .map(|(key, value)| Ok((key, value.try_into()?)))
                .collect::<Result<Map<String, JsonValue>, Error>>()?,
        ))
    }

    fn into_validating_json_value(self) -> Result<JsonValue, Error> {
        Ok(JsonValue::Object(
            self.into_iter()
                .map(|(key, value)| Ok((key, value.try_into_validating_json()?)))
                .collect::<Result<Map<String, JsonValue>, Error>>()?,
        ))
    }

    fn to_json_value(&self) -> Result<JsonValue, Error> {
        Ok(JsonValue::Object(
            self.iter()
                .map(|(key, value)| Ok((key.clone(), value.clone().try_into()?)))
                .collect::<Result<Map<String, JsonValue>, Error>>()?,
        ))
    }

    fn to_validating_json_value(&self) -> Result<JsonValue, Error> {
        Ok(JsonValue::Object(
            self.iter()
                .map(|(key, value)| Ok((key.to_owned(), value.try_to_validating_json()?)))
                .collect::<Result<Map<String, JsonValue>, Error>>()?,
        ))
    }

    fn from_json_value(value: JsonValue) -> Result<Self, Error> {
        let platform_value: Value = value.into();
        platform_value.into_btree_string_map()
    }
}

impl From<BTreeMap<String, JsonValue>> for Value {
    fn from(value: BTreeMap<String, JsonValue>) -> Self {
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

impl From<&BTreeMap<String, JsonValue>> for Value {
    fn from(value: &BTreeMap<String, JsonValue>) -> Self {
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
