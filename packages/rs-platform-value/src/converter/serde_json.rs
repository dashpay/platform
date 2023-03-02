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
                if !array.is_empty()
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
            Value::Bytes(bytes) => JsonValue::Array(
                bytes
                    .into_iter()
                    .map(|byte| JsonValue::Number(byte.into()))
                    .collect(),
            ),
            Value::Float(float) => JsonValue::Number(Number::from_f64(float).unwrap_or(0.into())),
            Value::Text(string) => JsonValue::String(string),
            Value::Bool(value) => JsonValue::Bool(value),
            Value::Null => JsonValue::Null,
            //todo support tags
            Value::Tag(_, _) => {
                return Err(Error::Unsupported("tags not yet supported".to_string()));
            }
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
        })
    }
}

pub trait BTreeValueJsonConverter {
    fn into_json_value(self) -> Result<JsonValue, Error>;
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

    fn from_json_value(value: JsonValue) -> Result<Self, Error> {
        let platform_value: Value = value.into();
        platform_value.into_btree_map()
    }
}
